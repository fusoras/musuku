use geico::manager::{AddOptions, Manager, UpdateOptions, UpdateOutcome};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn executable(path: &std::path::Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn test_add_fallback_name_creates_desktop_and_symlink() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("blender");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.name, "blender");
    assert!(entry.desktop_file.as_ref().unwrap().exists());
    assert!(entry.symlink.as_ref().unwrap().exists());
    let desktop = fs::read_to_string(entry.desktop_file.unwrap()).unwrap();
    assert!(desktop.contains("Type=Application"));
    assert!(desktop.contains("Name=blender"));
    assert!(desktop.contains("Exec="));
    assert!(desktop.contains("Terminal=false"));
}

#[test]
fn test_add_no_desktop_creates_only_symlink() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target,
            no_desktop: true,
            ..AddOptions::default()
        })
        .unwrap();

    assert!(entry.desktop_file.is_none());
    assert!(entry.symlink.as_ref().unwrap().exists());
}

#[test]
fn test_add_no_symlink_creates_only_desktop() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target,
            no_symlink: true,
            ..AddOptions::default()
        })
        .unwrap();

    assert!(entry.desktop_file.as_ref().unwrap().exists());
    assert!(entry.symlink.is_none());
}

#[test]
fn test_remove_deletes_generated_files_but_preserves_original_binary() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let desktop = entry.desktop_file.unwrap();
    let symlink = entry.symlink.unwrap();

    manager.remove(&entry.id).unwrap();

    assert!(!desktop.exists());
    assert!(!symlink.exists());
    assert!(
        target.exists(),
        "CRITICAL: original binary must never be deleted"
    );
}

#[test]
fn test_update_retargets_desktop_and_symlink() {
    let temp = tempdir().unwrap();
    let old_target = temp.path().join("old-tool");
    let new_target = temp.path().join("new-tool");
    executable(&old_target);
    executable(&new_target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: old_target,
            name: Some("Tool".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let outcome = manager
        .update(
            &entry.id,
            UpdateOptions {
                target: Some(new_target.clone()),
                name: Some("Renamed".to_string()),
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    let updated = match outcome {
        UpdateOutcome::Updated { entry } => entry,
        UpdateOutcome::NoChanges { .. } => panic!("expected update outcome"),
    };
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.target_bin, new_target);
    assert!(updated.desktop_file.unwrap().exists());
    assert!(updated.symlink.unwrap().exists());
}

#[test]
fn test_update_with_no_options_returns_no_changes_outcome_and_does_not_mutate() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            name: Some("Tool".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let outcome = manager.update(&entry.id, UpdateOptions::default()).unwrap();

    assert!(matches!(outcome, UpdateOutcome::NoChanges { .. }));
}

#[test]
fn test_list_formats_id_with_dim_gray_color() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("pepomote");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("PepoMote".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.list().unwrap();
    let expected_id_badge = format!(
        "{}({}){}",
        geico::colors::DIM_GRAY,
        "pepomote",
        geico::colors::RESET
    );
    assert!(
        output.contains(&expected_id_badge),
        "Expected list output to contain dim gray id badge: {expected_id_badge}, but got: {output}"
    );
}
