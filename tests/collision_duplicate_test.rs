//! Regression tests for the duplicate-registration collision bug.
//!
//! Reproduces: an app registered under two different IDs pointing at the
//! same target binary (e.g. after moving the AppImage and re-adding it).
//! Choosing "overwrite" used to remove only the ID-colliding entry, leaving
//! the second registration and its `.desktop` launcher behind — two menu
//! entries opening the same program.

use musu::manager::{AddOptions, Manager};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::tempdir;

fn executable(path: &Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn manager_with(base: &Path) -> Manager {
    Manager::new(
        base.join("config/apps.toml"),
        base.join("share"),
        base.join("bin"),
    )
}

/// Counts the `.desktop` files present in the manager's applications dir.
fn desktop_files(manager_data: &Path) -> Vec<String> {
    let applications = manager_data.join("applications");
    let mut files: Vec<String> = fs::read_dir(applications)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.ends_with(".desktop"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[test]
fn test_detect_add_collision_flags_same_target_under_different_id() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("OpenShot.AppImage");
    executable(&target);
    let manager = manager_with(temp.path());

    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("openshot".into()),
            no_desktop: true,
            no_symlink: true,
            ..Default::default()
        })
        .unwrap();

    // Same binary, different display name -> different ID.
    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    let new_entry = musu::models::AppEntry::new(
        Some("OpenShot Video Editor"),
        fs::canonicalize(&target).unwrap(),
    );
    let collision =
        musu::manager::detect_add_collision(&registry, &temp.path().join("share"), &new_entry)
            .expect("same target under another ID must be reported as a collision");

    assert_eq!(collision.duplicates.len(), 1);
    assert_eq!(collision.duplicates[0].id, "openshot");
}

#[test]
fn test_detect_add_collision_ignores_distinct_targets() {
    let temp = tempdir().unwrap();
    let target_a = temp.path().join("a.AppImage");
    let target_b = temp.path().join("b.AppImage");
    executable(&target_a);
    executable(&target_b);
    let manager = manager_with(temp.path());

    manager
        .add(AddOptions {
            target: target_a,
            name: Some("App A".into()),
            no_desktop: true,
            no_symlink: true,
            ..Default::default()
        })
        .unwrap();

    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    let new_entry =
        musu::models::AppEntry::new(Some("App B"), fs::canonicalize(&target_b).unwrap());
    assert!(
        musu::manager::detect_add_collision(&registry, &temp.path().join("share"), &new_entry)
            .is_none(),
        "distinct targets must not be treated as duplicates"
    );
}

#[test]
fn test_overwrite_removes_duplicate_registration_and_desktop_file() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("Appimage");
    let downloads = temp.path().join("Downloads");
    fs::create_dir_all(&app_dir).unwrap();
    fs::create_dir_all(&downloads).unwrap();

    // The AppImage lives in Appimage/ and a copy lived in Downloads/.
    let moved_target = app_dir.join("OpenShot.AppImage");
    let old_target = downloads.join("OpenShot.AppImage");
    executable(&moved_target);
    executable(&old_target);

    let manager = manager_with(temp.path());
    let data_dir = temp.path().join("share");

    // First registration: the AppImage already in Appimage/, short name.
    manager
        .add(AddOptions {
            target: moved_target.clone(),
            name: Some("openshot".into()),
            ..Default::default()
        })
        .unwrap();

    // Second registration: the Downloads copy (a different file at the
    // time), long name.
    manager
        .add(AddOptions {
            target: old_target.clone(),
            name: Some("OpenShot Video Editor".into()),
            ..Default::default()
        })
        .unwrap();

    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    assert_eq!(registry.apps.len(), 2, "both registrations must exist");
    assert_eq!(desktop_files(&data_dir).len(), 2);

    // The user deletes the Downloads copy manually.
    fs::remove_file(&old_target).unwrap();

    // Re-add the moved AppImage and answer "overwrite".
    manager
        .add(AddOptions {
            target: moved_target.clone(),
            name: Some("OpenShot Video Editor".into()),
            overwrite: true,
            ..Default::default()
        })
        .unwrap();

    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    assert_eq!(
        registry.apps.len(),
        1,
        "overwrite must leave a single registration, got: {:?}",
        registry.apps.iter().map(|a| &a.id).collect::<Vec<_>>()
    );

    let remaining = &registry.apps[0];
    assert_eq!(remaining.id, "openshot-video-editor");
    assert_eq!(
        fs::canonicalize(&remaining.target_bin).unwrap(),
        fs::canonicalize(&moved_target).unwrap(),
        "remaining entry must point at the moved AppImage"
    );

    assert_eq!(
        desktop_files(&data_dir),
        vec!["openshot-video-editor.desktop".to_string()],
        "the stale openshot.desktop must be gone"
    );

    // Non-destructive invariant: the target binary is never deleted.
    assert!(moved_target.exists(), "target binary must never be deleted");
}

#[test]
fn test_add_without_overwrite_reports_duplicate_noninteractively() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("app.AppImage");
    executable(&target);
    let manager = manager_with(temp.path());

    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("short-name".into()),
            no_desktop: true,
            no_symlink: true,
            ..Default::default()
        })
        .unwrap();

    let err = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("A Longer Different Name".into()),
            no_desktop: true,
            no_symlink: true,
            ..Default::default()
        })
        .unwrap_err();

    assert!(
        err.to_string().contains("already exists"),
        "non-interactive add on a duplicate target must error, got: {err}"
    );
}
