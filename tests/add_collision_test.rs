use musu::manager::{
    AddOptions, CollisionAction, Manager, detect_add_collision, parse_collision_choice,
};
use musu::models::AppEntry;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::tempdir;

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn manager_with(temp: &tempfile::TempDir) -> Manager {
    Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    )
}

fn registry_of(temp: &tempfile::TempDir) -> musu::models::AppRegistry {
    musu::config::load(&temp.path().join("config/apps.toml")).unwrap()
}

#[test]
fn test_parse_collision_choice_defaults_to_cancel() {
    assert_eq!(parse_collision_choice(""), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("   "), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("c"), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("C"), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("cancel"), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("garbage"), CollisionAction::Cancel);
    assert_eq!(parse_collision_choice("r"), CollisionAction::Rename);
    assert_eq!(parse_collision_choice("R"), CollisionAction::Rename);
    assert_eq!(parse_collision_choice("rename"), CollisionAction::Rename);
    assert_eq!(parse_collision_choice("o"), CollisionAction::Overwrite);
    assert_eq!(parse_collision_choice("O"), CollisionAction::Overwrite);
    assert_eq!(
        parse_collision_choice("overwrite"),
        CollisionAction::Overwrite
    );
    assert_eq!(
        parse_collision_choice("replace"),
        CollisionAction::Overwrite
    );
}

#[test]
fn test_detect_add_collision_with_registry_entry() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);
    let data_dir = temp.path().join("share");
    let target = temp.path().join("zen");
    executable(&target);
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    let registry = registry_of(&temp);
    let collision = detect_add_collision(&registry, &data_dir, &entry).unwrap();
    assert_eq!(collision.existing.as_ref().unwrap().id, entry.id);
    assert_eq!(
        collision.desktop_file,
        Some(data_dir.join("applications").join("zen.desktop"))
    );
    assert_eq!(collision.label(), "zen");
}

#[test]
fn test_detect_add_collision_orphaned_desktop_only() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("share");
    let target = temp.path().join("zen");
    executable(&target);
    let entry = AppEntry::new(Some("Zen"), target);

    // Orphaned desktop file on disk, empty registry.
    let applications = data_dir.join("applications");
    fs::create_dir_all(&applications).unwrap();
    let desktop = applications.join("zen.desktop");
    fs::write(&desktop, "[Desktop Entry]\n").unwrap();

    let registry = registry_of(&temp);
    let collision = detect_add_collision(&registry, &data_dir, &entry).unwrap();
    assert!(collision.existing.is_none());
    assert_eq!(collision.desktop_file, Some(desktop));
    assert_eq!(collision.label(), "zen");
}

#[test]
fn test_detect_add_collision_none_when_name_is_free() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("share");
    let target = temp.path().join("zen");
    executable(&target);
    let entry = AppEntry::new(Some("Zen"), target);
    let registry = registry_of(&temp);
    assert!(detect_add_collision(&registry, &data_dir, &entry).is_none());
}

#[test]
fn test_add_duplicate_without_overwrite_fails_and_preserves() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);
    let target = temp.path().join("zen");
    executable(&target);
    manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    // Non-interactive sessions (no TTY) cancel safely by default.
    let err = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap_err();
    assert!(format!("{err:#}").contains("already exists"));

    let registry = registry_of(&temp);
    assert_eq!(registry.apps.len(), 1);
    assert!(registry.apps[0].desktop_file.as_ref().unwrap().exists());
    assert!(registry.apps[0].symlink.as_ref().unwrap().exists());
}

#[test]
fn test_add_with_different_name_coexists_without_collision() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);
    let zen = temp.path().join("zen");
    let zen_beta = temp.path().join("zen-beta");
    executable(&zen);
    executable(&zen_beta);

    let first = manager
        .add(AddOptions {
            target: zen.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let second = manager
        .add(AddOptions {
            target: zen_beta.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(first.id, "zen");
    assert_eq!(second.id, "zen-beta");
    let registry = registry_of(&temp);
    assert_eq!(registry.apps.len(), 2);
}

#[test]
fn test_add_overwrite_replaces_existing_registration() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);
    let old_target = temp.path().join("zen-old");
    let new_target = temp.path().join("zen-new");
    executable(&old_target);
    executable(&new_target);

    let first = manager
        .add(AddOptions {
            target: old_target.clone(),
            name: Some("Zen".to_string()),
            ..AddOptions::default()
        })
        .unwrap();
    let old_desktop = first.desktop_file.clone().unwrap();
    let old_symlink = first.symlink.clone().unwrap();

    let second = manager
        .add(AddOptions {
            target: new_target.clone(),
            name: Some("Zen".to_string()),
            overwrite: true,
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(second.id, first.id);
    assert_eq!(second.target_bin, new_target);

    // Exactly one registry entry remains, pointing at the new target.
    let registry = registry_of(&temp);
    assert_eq!(registry.apps.len(), 1);
    assert_eq!(registry.apps[0].target_bin, new_target);

    // Desktop launcher was rewritten for the new target.
    let contents = fs::read_to_string(&old_desktop).unwrap();
    assert!(contents.contains(new_target.to_str().unwrap()));
    // The desktop path is stable (same ID), the symlink is regenerated.
    assert_eq!(second.desktop_file, Some(old_desktop));
    assert_eq!(second.symlink, Some(old_symlink));
    assert!(second.symlink.as_ref().unwrap().exists());

    // Original target binaries are never deleted (non-destructive).
    assert!(old_target.exists());
    assert!(new_target.exists());
}

#[test]
fn test_add_overwrite_cleans_orphaned_desktop() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);
    let data_dir = temp.path().join("share");
    let target = temp.path().join("zen");
    executable(&target);

    // Orphaned desktop file with no registry entry.
    let applications = data_dir.join("applications");
    fs::create_dir_all(&applications).unwrap();
    let desktop = applications.join("zen.desktop");
    fs::write(&desktop, "[Desktop Entry]\nName=Stale\n").unwrap();

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            overwrite: true,
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.desktop_file, Some(desktop.clone()));
    let contents = fs::read_to_string(&desktop).unwrap();
    assert!(contents.contains("zen"));
    assert!(!contents.contains("Stale"));
}
