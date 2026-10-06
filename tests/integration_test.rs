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

#[test]
fn test_add_relative_target_path_is_canonicalized_to_absolute() {
    let temp = tempdir().unwrap();
    let rel_target = std::path::PathBuf::from("target/test_rel_tool");
    executable(&rel_target);

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target: rel_target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    assert!(
        entry.target_bin.is_absolute(),
        "Expected target_bin to be canonicalized to an absolute path, got: {:?}",
        entry.target_bin
    );
    assert_eq!(
        entry.name, "test_rel_tool",
        "Display name should still derive from the filename"
    );

    let symlink = entry.symlink.expect("symlink should be created");
    assert!(symlink.exists(), "Symlink in bin_dir should exist");
    let link_dest = fs::read_link(&symlink).unwrap();
    assert!(
        link_dest.is_absolute(),
        "Symlink destination should be absolute, got: {:?}",
        link_dest
    );

    let _ = fs::remove_file(&rel_target);
}

#[test]
fn test_update_name_updates_symlink_name_when_using_default_id() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("mytool");
    executable(&target);

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target,
            name: Some("Old Tool".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.id, "old-tool");
    let old_symlink = temp.path().join("bin/old-tool");
    assert!(old_symlink.exists());

    let outcome = manager
        .update(
            "old-tool",
            UpdateOptions {
                name: Some("New Tool".to_string()),
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    let updated = match outcome {
        UpdateOutcome::Updated { entry } => entry,
        UpdateOutcome::NoChanges { .. } => panic!("expected updated entry"),
    };

    assert_eq!(updated.id, "new-tool");
    assert!(
        !old_symlink.exists(),
        "Old symlink should have been removed"
    );
    let new_symlink = temp.path().join("bin/new-tool");
    assert!(new_symlink.exists(), "New symlink should have been created");
}

#[test]
fn test_update_categories_modifies_desktop_file_and_normalizes() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("shotcut");
    executable(&target);

    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target,
            name: Some("Shotcut".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let outcome = manager
        .update(
            &entry.id,
            UpdateOptions {
                categories: Some(vec!["video".to_string(), "audiovideo".to_string()]),
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    let updated = match outcome {
        UpdateOutcome::Updated { entry } => entry,
        UpdateOutcome::NoChanges { .. } => panic!("expected updated entry"),
    };

    assert_eq!(
        updated.categories,
        Some(vec!["Video".to_string(), "AudioVideo".to_string()])
    );
    let desktop_content = fs::read_to_string(updated.desktop_file.unwrap()).unwrap();
    assert!(
        desktop_content.contains("Categories=Video;AudioVideo;\n"),
        "Expected Categories in .desktop file, got:\n{desktop_content}"
    );
}
