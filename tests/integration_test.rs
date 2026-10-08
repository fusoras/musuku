use clap::Parser;
use musu::cli::{Cli, Command};
use musu::manager::{
    AddOptions, Manager, RemoveOptions, ShowOptions, UpdateOptions, UpdateOutcome,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn executable(path: &std::path::Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn appimage(path: &std::path::Path) {
    executable(path);
}

fn manager_with_settings(temp: &tempfile::TempDir) -> Manager {
    Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    )
}

fn visible_len(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut width = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            width += 1;
            index += 1;
            continue;
        }
        index += 1;
        if bytes.get(index) == Some(&b'_') {
            while index < bytes.len() {
                if bytes[index] == 0x1b && bytes.get(index + 1) == Some(&b'\\') {
                    index += 2;
                    break;
                }
                index += 1;
            }
        } else {
            while index < bytes.len() && !bytes[index].is_ascii_alphabetic() {
                index += 1;
            }
            index += usize::from(index < bytes.len());
        }
    }
    width
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
fn test_config_disables_symlinks_for_appimages_but_keeps_desktop_and_icon() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("video.AppImage");
    let icon = temp.path().join("video.png");
    appimage(&target);
    fs::write(&icon, b"icon").unwrap();
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[defaults]\nsymlink_appimages = false\n",
    )
    .unwrap();

    let entry = manager
        .add(AddOptions {
            target,
            icon: Some(icon),
            name: Some("Video".into()),
            categories: Some(vec!["Video".into()]),
            ..AddOptions::default()
        })
        .unwrap();

    assert!(
        entry
            .desktop_file
            .as_ref()
            .is_some_and(|path| path.exists())
    );
    assert!(entry.icon_path.as_ref().is_some_and(|path| path.exists()));
    assert!(entry.symlink.is_none());
}

#[test]
fn test_config_disables_appimage_symlinks_only() {
    let temp = tempdir().unwrap();
    let appimage_target = temp.path().join("video.AppImage");
    let script_target = temp.path().join("tool.sh");
    appimage(&appimage_target);
    executable(&script_target);
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[defaults]\nappimage_symlinks = false\n",
    )
    .unwrap();

    let appimage_entry = manager
        .add(AddOptions {
            target: appimage_target,
            name: Some("Video".into()),
            no_desktop: true,
            ..AddOptions::default()
        })
        .unwrap();
    let script_entry = manager
        .add(AddOptions {
            target: script_target,
            name: Some("Tool".into()),
            no_desktop: true,
            ..AddOptions::default()
        })
        .unwrap();

    assert!(appimage_entry.symlink.is_none());
    assert!(
        script_entry
            .symlink
            .as_ref()
            .is_some_and(|path| path.exists())
    );
}

#[test]
fn test_appimage_symlink_defaults_to_disabled_but_config_can_enable_it() {
    for config_contents in [None, Some("[defaults]\nsymlink_appimages = true\n")] {
        let should_exist = config_contents.is_some();
        let temp = tempdir().unwrap();
        let target = temp.path().join("video.AppImage");
        appimage(&target);
        let manager = manager_with_settings(&temp);
        if let Some(contents) = config_contents {
            fs::create_dir_all(temp.path().join("config")).unwrap();
            fs::write(temp.path().join("config/config.toml"), contents).unwrap();
        }

        let entry = manager
            .add(AddOptions {
                target,
                categories: Some(vec![]),
                no_desktop: true,
                ..AddOptions::default()
            })
            .unwrap();
        assert_eq!(
            entry.symlink.as_ref().is_some_and(|path| path.exists()),
            should_exist
        );
    }
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
fn test_remove_with_delete_target_deletes_executable_from_disk() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let icon = temp.path().join("tool.png");
    fs::write(&icon, b"icon").unwrap();
    let manager = manager_with_settings(&temp);
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            icon: Some(icon),
            ..AddOptions::default()
        })
        .unwrap();
    let desktop = entry.desktop_file.clone().unwrap();
    let symlink_path = entry.symlink.clone().unwrap();
    let icon_path = entry.icon_path.clone().unwrap();

    manager
        .remove_with_options(
            &entry.id,
            RemoveOptions {
                delete_target: true,
            },
        )
        .unwrap();

    assert!(!desktop.exists());
    assert!(!icon_path.exists());
    assert!(!symlink_path.exists());
    assert!(
        !target.exists(),
        "delete_target must remove the executable from disk"
    );
    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    assert!(registry.find(&entry.id).is_none());
}

#[test]
fn test_remove_default_still_preserves_executable_from_disk() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let desktop = entry.desktop_file.clone().unwrap();
    let symlink_path = entry.symlink.clone().unwrap();

    manager.remove(&entry.id).unwrap();

    assert!(!desktop.exists());
    assert!(!symlink_path.exists());
    assert!(
        target.exists(),
        "CRITICAL: remove without --delete-target must never delete the target"
    );
    assert!(
        manager
            .remove_with_options(&entry.id, RemoveOptions::default())
            .is_err(),
        "entry must be gone from the registry"
    );
}

#[test]
fn test_remove_delete_target_without_yes_on_noninteractive_stdin_aborts_with_error() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        xdg_config.join("musu/apps.toml"),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let desktop = entry.desktop_file.clone().unwrap();
    let symlink_path = entry.symlink.clone().unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_musu"))
        .args(["remove", &entry.id, "--delete-target"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("XDG_DATA_HOME", &xdg_data)
        .env_remove("SHELL")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "non-interactive confirmation must exit with a non-zero status; stderr: {stderr}"
    );
    assert!(
        stderr.contains("--yes"),
        "error must tell the user how to confirm non-interactively; stderr: {stderr}"
    );
    assert!(
        target.exists(),
        "CRITICAL: target binary must never be deleted"
    );
    assert!(desktop.exists(), "abort must happen before any deletion");
    assert!(
        symlink_path.exists(),
        "abort must happen before any deletion"
    );
    let registry = musu::config::load(&xdg_config.join("musu/apps.toml")).unwrap();
    assert!(
        registry.find(&entry.id).is_some(),
        "registry must be untouched after abort"
    );
}

#[test]
fn test_remove_accepts_multiple_applications() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let manager = Manager::new(
        xdg_config.join("musu/apps.toml"),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    let first_target = temp.path().join("first");
    let second_target = temp.path().join("second");
    executable(&first_target);
    executable(&second_target);
    let first = manager
        .add(AddOptions {
            target: first_target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let second = manager
        .add(AddOptions {
            target: second_target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_musu"))
        .args(["remove", &first.id, &second.id])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("XDG_DATA_HOME", &xdg_data)
        .env_remove("SHELL")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "removing multiple applications failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let registry = musu::config::load(&xdg_config.join("musu/apps.toml")).unwrap();
    assert!(registry.apps.is_empty());
    assert!(first_target.exists());
    assert!(second_target.exists());
}

#[test]
fn test_remove_all_with_yes_removes_every_registered_application() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let manager = Manager::new(
        xdg_config.join("musu/apps.toml"),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    let first_target = temp.path().join("first");
    let second_target = temp.path().join("second");
    executable(&first_target);
    executable(&second_target);
    manager
        .add(AddOptions {
            target: first_target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    manager
        .add(AddOptions {
            target: second_target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_musu"))
        .args(["remove", "--all", "-y"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("XDG_DATA_HOME", &xdg_data)
        .env_remove("SHELL")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "remove --all failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let registry = musu::config::load(&xdg_config.join("musu/apps.toml")).unwrap();
    assert!(registry.apps.is_empty());
    assert!(first_target.exists());
    assert!(second_target.exists());
}

#[test]
fn test_remove_all_without_yes_requires_interactive_stdin() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = Manager::new(
        xdg_config.join("musu/apps.toml"),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_musu"))
        .args(["remove", "--all"])
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("XDG_DATA_HOME", &xdg_data)
        .env_remove("SHELL")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("--yes"), "stderr: {stderr}");
    assert!(target.exists());
    let registry = musu::config::load(&xdg_config.join("musu/apps.toml")).unwrap();
    assert!(registry.find(&entry.id).is_some());
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
        UpdateOutcome::Updated { new: entry, .. } => entry,
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
fn test_update_remove_symlink_deletes_symlink_file_and_updates_registry() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            ..AddOptions::default()
        })
        .unwrap();
    let desktop = entry.desktop_file.clone().unwrap();
    let symlink_path = entry.symlink.clone().unwrap();
    assert!(symlink_path.exists());

    let outcome = manager
        .update(
            &entry.id,
            UpdateOptions {
                remove_symlink: true,
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    let updated = match outcome {
        UpdateOutcome::Updated { new: entry, .. } => entry,
        UpdateOutcome::NoChanges { .. } => panic!("expected updated outcome after symlink removal"),
    };
    assert!(
        !symlink_path.exists(),
        "symlink file must be deleted from disk"
    );
    assert!(updated.symlink.is_none());
    let registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    let stored = registry.find(&entry.id).unwrap();
    assert!(
        stored.symlink.is_none(),
        "apps.toml must record symlink = None"
    );
    assert!(desktop.exists(), "desktop launcher must remain untouched");
    assert!(
        target.exists(),
        "CRITICAL: target binary must never be deleted"
    );
}

#[test]
fn test_update_remove_symlink_when_none_returns_no_changes() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    let entry = manager
        .add(AddOptions {
            target,
            no_symlink: true,
            ..AddOptions::default()
        })
        .unwrap();
    assert!(entry.symlink.is_none());

    let outcome = manager
        .update(
            &entry.id,
            UpdateOptions {
                remove_symlink: true,
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    assert!(
        matches!(outcome, UpdateOutcome::NoChanges { .. }),
        "removing a symlink that does not exist must report NoChanges"
    );
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
        musu::colors::DIM_GRAY,
        "pepomote",
        musu::colors::RESET
    );
    assert!(
        output.contains(&expected_id_badge),
        "Expected list output to contain dim gray id badge: {expected_id_badge}, but got: {output}"
    );
}

#[test]
fn test_list_table_format_contains_headers() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("table-tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    manager
        .add(AddOptions {
            target,
            name: Some("Table Tool".into()),
            ..AddOptions::default()
        })
        .unwrap();
    let output = manager
        .list_with_options(musu::manager::ListOptions {
            table: true,
            ..Default::default()
        })
        .unwrap();
    assert!(output.contains("NAME"));
    assert!(output.contains("ID"));
    assert!(output.contains("STATUS"));
}

#[test]
fn test_list_filter_by_category_and_broken() {
    let temp = tempdir().unwrap();
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let video = temp.path().join("video");
    executable(&video);
    let video_entry = manager
        .add(AddOptions {
            target: video,
            name: Some("Video Tool".into()),
            categories: Some(vec!["Video".into()]),
            ..AddOptions::default()
        })
        .unwrap();
    let output = manager
        .list_with_options(musu::manager::ListOptions {
            category: Some("video".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(output.contains(&video_entry.name));
    assert!(!output.contains("Missing Tool"));

    let mut registry = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    registry.add(musu::models::AppEntry::new(
        Some("Missing Tool"),
        temp.path().join("missing"),
    ));
    musu::config::save(&temp.path().join("config/apps.toml"), &registry).unwrap();
    let broken = manager
        .list_with_options(musu::manager::ListOptions {
            broken: true,
            ..Default::default()
        })
        .unwrap();
    assert!(broken.contains("Missing Tool"));
    assert!(!broken.contains(&video_entry.name));
}

#[test]
fn test_run_subcommand_locates_and_launches() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("runner");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            name: Some("Runner".into()),
            no_desktop: true,
            no_symlink: true,
            ..AddOptions::default()
        })
        .unwrap();
    manager.run(&entry.id, &[]).unwrap();
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
        UpdateOutcome::Updated { new: entry, .. } => entry,
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
        UpdateOutcome::Updated { new: entry, .. } => entry,
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

#[test]
fn test_show_default_formats_application_card() {
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
            name: Some("Test Tool".to_string()),
            categories: Some(vec!["Development".to_string(), "Utility".to_string()]),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, ShowOptions::default()).unwrap();

    assert!(output.contains("Test Tool (test-tool)"));
    assert!(output.contains("Executable:"));
    assert!(output.contains("Desktop:"));
    assert!(output.contains("Icon:"));
    assert!(output.contains("Symlink:"));
    assert!(output.contains("Categories:  Development, Utility"));
    assert!(output.contains("Registered:"));
    assert!(output.contains("ok"));
}

#[test]
fn test_show_single_path_flags_return_only_requested_path() {
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
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                bin: true,
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(output, entry.target_bin.display().to_string());
}

#[test]
fn test_show_json_returns_serialized_application_entry() {
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
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                json: true,
                ..Default::default()
            },
        )
        .unwrap();
    let serialized: serde_json::Value = serde_json::from_str(&output).unwrap();

    assert_eq!(serialized["id"], entry.id);
    assert_eq!(serialized["name"], entry.name);
    assert_eq!(
        serialized["target_bin"],
        entry.target_bin.display().to_string()
    );
}

#[test]
fn test_show_nonexistent_application_returns_error() {
    let temp = tempdir().unwrap();
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let error = manager
        .show("missing", ShowOptions::default())
        .unwrap_err()
        .to_string();

    assert_eq!(error, "Application 'missing' not found.");
}

#[test]
fn test_show_no_image_flag_outputs_text_card() {
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
            name: Some("Test Tool".to_string()),
            categories: Some(vec!["Development".to_string()]),
            ..AddOptions::default()
        })
        .unwrap();

    let cli = Cli::try_parse_from(["musu", "show", entry.id.as_str(), "--no-icons"])
        .expect("musu show must accept the --no-icons flag");
    let args = match cli.command {
        Command::Show(args) => args,
        other => panic!("expected the show subcommand, got {other:?}"),
    };
    assert!(args.no_icons, "--no-icons must be recorded by the CLI");

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                no_icons: args.no_icons,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(
        output.contains("Test Tool (test-tool)"),
        "expected the text card header, got: {output}"
    );
    assert!(output.contains("Executable:"), "got: {output}");
    assert!(output.contains("Desktop:"), "got: {output}");
    assert!(output.contains("Icon:"), "got: {output}");
    assert!(output.contains("Symlink:"), "got: {output}");
    assert!(output.contains("Categories:  Development"), "got: {output}");
    assert!(output.contains("Registered:"), "got: {output}");
}

#[test]
fn test_list_cli_accepts_image_and_no_image_flags() {
    let cli_icons = Cli::try_parse_from(["musu", "list", "--icons"])
        .expect("musu list must accept --icons flag");
    match cli_icons.command {
        Command::List(args) => assert!(args.icons),
        other => panic!("expected List command, got {other:?}"),
    }

    let cli_img = Cli::try_parse_from(["musu", "list", "--image"])
        .expect("musu list must accept --image alias flag");
    match cli_img.command {
        Command::List(args) => assert!(args.icons),
        other => panic!("expected List command, got {other:?}"),
    }

    let cli_short =
        Cli::try_parse_from(["musu", "list", "-i"]).expect("musu list must accept -i flag");
    match cli_short.command {
        Command::List(args) => assert!(args.icons),
        other => panic!("expected List command, got {other:?}"),
    }

    let cli_no_icons = Cli::try_parse_from(["musu", "list", "--no-icons"])
        .expect("musu list must accept --no-icons flag");
    match cli_no_icons.command {
        Command::List(args) => assert!(args.no_icons),
        other => panic!("expected List command, got {other:?}"),
    }

    let cli_no_img = Cli::try_parse_from(["musu", "list", "--no-image"])
        .expect("musu list must accept --no-image flag");
    match cli_no_img.command {
        Command::List(args) => assert!(args.no_icons),
        other => panic!("expected List command, got {other:?}"),
    }

    let cli_hide = Cli::try_parse_from(["musu", "list", "--hide-disabled-symlinks"])
        .expect("musu list must accept --hide-disabled-symlinks");
    match cli_hide.command {
        Command::List(args) => assert!(args.hide_disabled_symlinks),
        other => panic!("expected List command, got {other:?}"),
    }
}

#[test]
fn test_list_with_image_flag_outputs_image_preview() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool.sh");
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"dummy-png-data").unwrap();

    let manager = Manager::new(
        temp.path().join("apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    manager
        .add(AddOptions {
            target,
            name: Some("ImageApp".to_string()),
            icon: Some(icon),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .list_with_options(musu::manager::ListOptions {
            image: true,
            no_image: false,
            ..Default::default()
        })
        .unwrap();

    assert!(
        output.contains("\x1b_Ga=T,f=100"),
        "musu list with image=true should contain Kitty escape sequence, got: {output}"
    );
}

#[test]
fn test_list_with_image_flag_inline_compact_thumbnail_on_left() {
    let temp = tempdir().unwrap();
    let target1 = temp.path().join("app1.sh");
    executable(&target1);
    let icon1 = temp.path().join("icon1.png");
    fs::write(&icon1, b"dummy-png-data").unwrap();

    let target2 = temp.path().join("app2.sh");
    executable(&target2);

    let manager = Manager::new(
        temp.path().join("apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    manager
        .add(AddOptions {
            target: target1,
            name: Some("AppWithIcon".to_string()),
            icon: Some(icon1),
            ..AddOptions::default()
        })
        .unwrap();

    manager
        .add(AddOptions {
            target: target2,
            name: Some("AppNoIcon".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .list_with_options(musu::manager::ListOptions {
            image: true,
            no_image: false,
            ..Default::default()
        })
        .unwrap();

    // Verify compact 2x1 dimensions in Kitty escape sequence
    assert!(
        output.contains("c=2,r=1"),
        "Expected compact 2x1 thumbnail dimensions c=2,r=1, got: {output}"
    );

    // Verify icon is on the left side of the name (inline with space), NOT on a separate line above
    assert!(
        output.contains("\x1b\\ AppWithIcon"),
        "Expected icon to be placed on the left side of the name, got: {output}"
    );

    // Verify apps without icon have leading spaces for column alignment
    assert!(
        output.contains("   AppNoIcon"),
        "Expected apps without icon to have leading spaces for alignment, got: {output}"
    );
}

#[test]
fn test_show_with_image_flag_forces_preview() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("tool.sh");
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"dummy-png-data").unwrap();

    let manager = Manager::new(
        temp.path().join("apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target,
            name: Some("ShowImageApp".to_string()),
            icon: Some(icon),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                image: true,
                ..Default::default()
            },
        )
        .unwrap();

    assert!(
        output.contains("\x1b_Ga=T,f=100"),
        "musu show with image=true should contain Kitty escape sequence even in non-TTY test, got: {output}"
    );
}

#[test]
fn test_show_collapses_home_directory_to_tilde_by_default() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("fakehome");
    let apps_dir = home.join("Apps");
    fs::create_dir_all(&apps_dir).unwrap();

    let target = apps_dir.join("sample_tool");
    executable(&target);

    let manager = Manager::new(
        home.join(".config/musu/apps.toml"),
        home.join(".local/share"),
        home.join(".local/bin"),
    )
    .with_home(home);

    let entry = manager
        .add(AddOptions {
            target,
            name: Some("SampleTool".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, ShowOptions::default()).unwrap();

    assert!(
        output.contains("~/Apps/sample_tool"),
        "Expected show to collapse $HOME into ~/ by default, got: {output}"
    );
    assert!(
        output.contains("~/.local/share/applications/sampletool.desktop"),
        "Expected desktop path to collapse into ~/.local/share, got: {output}"
    );
    assert!(
        output.contains("~/.local/bin/sampletool"),
        "Expected symlink path to collapse into ~/.local/bin, got: {output}"
    );
}

#[test]
fn test_show_with_absolute_flag_prints_full_path() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("fakehome");
    let apps_dir = home.join("Apps");
    fs::create_dir_all(&apps_dir).unwrap();

    let target = apps_dir.join("sample_tool");
    executable(&target);

    let manager = Manager::new(
        home.join(".config/musu/apps.toml"),
        home.join(".local/share"),
        home.join(".local/bin"),
    )
    .with_home(home.clone());

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("SampleTool".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                absolute: true,
                ..Default::default()
            },
        )
        .unwrap();

    let expected_target = target.display().to_string();
    assert!(
        output.contains(&expected_target),
        "Expected show with absolute=true to print full path {expected_target}, got: {output}"
    );
}

#[test]
fn test_show_cli_accepts_absolute_and_full_path_flags() {
    let cli_abs = Cli::try_parse_from(["musu", "show", "app", "--absolute"])
        .expect("musu show must accept --absolute");
    match cli_abs.command {
        Command::Show(args) => assert!(args.absolute),
        other => panic!("expected Show command, got {other:?}"),
    }

    let cli_full = Cli::try_parse_from(["musu", "show", "app", "--full-path"])
        .expect("musu show must accept --full-path alias");
    match cli_full.command {
        Command::Show(args) => assert!(args.absolute),
        other => panic!("expected Show command, got {other:?}"),
    }

    let cli_short =
        Cli::try_parse_from(["musu", "show", "app", "-a"]).expect("musu show must accept -a flag");
    match cli_short.command {
        Command::Show(args) => assert!(args.absolute),
        other => panic!("expected Show command, got {other:?}"),
    }
}

#[test]
fn test_show_formats_registered_timestamp_without_nanoseconds() {
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
            name: Some("TimestampTest".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, ShowOptions::default()).unwrap();

    let expected_time = entry.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
    assert!(
        output.contains(&format!("Registered:  {expected_time}")),
        "Expected show to format Registered as '{expected_time}' without nanoseconds, got: {output}"
    );

    let reg_line = output
        .lines()
        .find(|line| line.trim_start().starts_with("Registered:"))
        .expect("Registered line must exist");
    assert!(
        !reg_line.contains('.'),
        "Registered line should not contain fractional seconds / dot: {reg_line}"
    );
}

#[test]
fn test_display_list_settings_control_format_size_color_and_images() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("configured-tool");
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"dummy-png-data").unwrap();
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[display]\nimages = \"always\"\nlist_icon_size = \"large\"\ncolor = \"never\"\nlist_format = \"table\"\n",
    )
    .unwrap();
    manager
        .add(AddOptions {
            target,
            icon: Some(icon),
            name: Some("Configured Tool".into()),
            ..Default::default()
        })
        .unwrap();

    let output = manager.list().unwrap();
    assert!(output.contains("ICON"));
    assert!(output.contains("NAME"));
    assert!(output.contains("c=8,r=4"));
    assert!(
        !output.contains("\x1b["),
        "color=never must suppress ANSI output"
    );
    let lines: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with('|'))
        .collect();
    assert!(
        lines
            .windows(2)
            .all(|pair| visible_len(pair[0]) == visible_len(pair[1]))
    );
}

#[test]
fn test_display_image_never_overrides_explicit_image_flag() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("no-preview-tool");
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"dummy-png-data").unwrap();
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[display]\nimages = \"never\"\n",
    )
    .unwrap();
    let entry = manager
        .add(AddOptions {
            target,
            icon: Some(icon),
            ..Default::default()
        })
        .unwrap();

    let output = manager
        .show(
            &entry.id,
            ShowOptions {
                image: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(
        !output.contains("\x1b_Ga=T"),
        "images=never must suppress previews"
    );

    let output = manager
        .list_with_options(musu::manager::ListOptions {
            image: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        !output.contains("\x1b_Ga=T"),
        "images=never must suppress list previews even with --image"
    );
}

#[test]
fn test_display_show_size_and_absolute_paths_are_configurable() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("fake-home");
    let target = home.join("apps/tool");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"dummy-png-data").unwrap();
    let manager = Manager::new(
        home.join(".config/musu/apps.toml"),
        home.join(".local/share"),
        home.join(".local/bin"),
    )
    .with_home(home.clone());
    fs::create_dir_all(home.join(".config/musu")).unwrap();
    fs::write(
        home.join(".config/musu/config.toml"),
        "[display]\nimages = \"always\"\nshow_icon_size = \"compact\"\npaths = \"absolute\"\n",
    )
    .unwrap();
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            icon: Some(icon),
            ..Default::default()
        })
        .unwrap();

    let output = manager.show(&entry.id, ShowOptions::default()).unwrap();
    assert!(output.contains("c=6,r=3"));
    assert!(output.contains(&target.display().to_string()));
    assert!(!output.contains("~/apps/tool"));
}

#[test]
fn test_defaults_categories_and_dest_bin_apply_when_cli_omits_them() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("defaults-tool");
    executable(&target);
    let destination = temp.path().join("custom-bin");
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        format!(
            "[defaults]\ncategories = [\"Development\"]\ndest_bin = \"{}\"\n",
            destination.display()
        ),
    )
    .unwrap();

    let entry = manager
        .add(AddOptions {
            target,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(entry.categories, Some(vec!["Development".into()]));
    assert_eq!(entry.symlink.unwrap().parent(), Some(destination.as_path()));
}
// ---------------------------------------------------------------------------
// `--categories` query mode (flag present without a value) and the removal of
// the `musu categories` subcommand.
// ---------------------------------------------------------------------------

fn run_musu(
    home: &std::path::Path,
    xdg_config: &std::path::Path,
    xdg_data: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_musu"))
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", xdg_config)
        .env("XDG_DATA_HOME", xdg_data)
        .env_remove("SHELL")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn test_cli_categories_flag_without_value_parses_as_empty_list() {
    for cmd in [
        vec!["musu", "update", "my-app", "--categories"],
        vec!["musu", "add", "/tmp/tool", "--categories"],
    ] {
        let cli = Cli::try_parse_from(&cmd).unwrap_or_else(|e| panic!("{cmd:?} must parse: {e}"));
        match cli.command {
            Command::Update(args) => assert_eq!(
                args.categories,
                Some(vec![]),
                "update --categories without a value must parse as Some(vec![])"
            ),
            Command::Add(args) => assert_eq!(
                args.categories,
                Some(vec![]),
                "add --categories without a value must parse as Some(vec![])"
            ),
            other => panic!("expected update/add subcommand, got {other:?}"),
        }
    }
}

#[test]
fn test_cli_categories_flag_with_value_parses_delimited_list() {
    let cli = Cli::try_parse_from([
        "musu",
        "update",
        "my-app",
        "--categories",
        "Video,AudioVideo",
    ])
    .expect("musu update <app> --categories Video,AudioVideo must parse");
    match cli.command {
        Command::Update(args) => assert_eq!(
            args.categories,
            Some(vec!["Video".to_string(), "AudioVideo".to_string()])
        ),
        other => panic!("expected the update subcommand, got {other:?}"),
    }
}

#[test]
fn test_cli_categories_flag_without_value_does_not_swallow_help() {
    let error = Cli::try_parse_from(["musu", "update", "my-app", "--categories", "--help"])
        .expect_err("--categories with no value must not consume --help");
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::DisplayHelp,
        "--help after --categories must render the help text, got: {error}"
    );
}

#[test]
fn test_cli_categories_subcommand_is_removed() {
    let error = Cli::try_parse_from(["musu", "categories"])
        .expect_err("`musu categories` was removed in favor of `--categories` with no value");
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::InvalidSubcommand,
        "expected an unrecognized-subcommand error, got: {error}"
    );
}

#[test]
fn test_update_categories_query_prints_current_categories_without_mutating_registry() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("openshot");
    executable(&target);

    let registry_path = xdg_config.join("musu/apps.toml");
    let manager = Manager::new(
        registry_path.clone(),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    manager
        .add(AddOptions {
            target,
            name: Some("openshot".to_string()),
            categories: Some(vec!["Video".to_string(), "AudioVideo".to_string()]),
            ..AddOptions::default()
        })
        .unwrap();
    let registry_before = fs::read(&registry_path).unwrap();

    let output = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["update", "openshot", "--categories"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "query mode must exit 0; stderr: {stderr}"
    );
    assert!(
        stdout.contains("Current categories for 'openshot': Video,AudioVideo"),
        "expected current categories header, got: {stdout}"
    );
    assert!(
        stdout.contains("Standard FreeDesktop Application Categories"),
        "expected the standard categories table, got: {stdout}"
    );
    assert!(
        stdout.contains("Usage tip: musu update openshot --categories Video,AudioVideo"),
        "expected the update usage tip, got: {stdout}"
    );
    assert!(
        stdout
            .trim_end()
            .lines()
            .next_back()
            .is_some_and(|line| line
                .contains("Usage tip: musu update openshot --categories Video,AudioVideo")),
        "usage tip must be the last line, got: {stdout}"
    );

    let registry_after = fs::read(&registry_path).unwrap();
    assert_eq!(
        registry_before, registry_after,
        "query mode must not mutate the registry"
    );
}

#[test]
fn test_update_categories_query_reports_none_when_app_has_no_categories() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("plain");
    executable(&target);

    let registry_path = xdg_config.join("musu/apps.toml");
    let manager = Manager::new(
        registry_path.clone(),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    manager
        .add(AddOptions {
            target,
            ..AddOptions::default()
        })
        .unwrap();

    let output = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["update", "plain", "--categories"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "query mode must exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Current categories for 'plain': Utility"),
        "expected the configured default category, got: {stdout}"
    );
}

#[test]
fn test_update_categories_query_errors_for_unknown_app() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();

    let output = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["update", "missing-app", "--categories"],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "query mode must fail for an unknown app; stderr: {stderr}"
    );
    assert!(
        stderr.contains("Application 'missing-app' not found."),
        "expected the normal not-found error, got: {stderr}"
    );
}

#[test]
fn test_add_categories_query_prints_table_and_registers_nothing() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("query-tool");
    executable(&target);

    let output = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["add", target.to_str().unwrap(), "--categories"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "add query mode must exit 0; stderr: {stderr}"
    );
    assert!(
        stdout.contains("Standard FreeDesktop Application Categories"),
        "expected the standard categories table, got: {stdout}"
    );
    assert!(
        stdout.contains("Usage tip: musu add <path> --categories Video,AudioVideo"),
        "expected the add usage tip, got: {stdout}"
    );
    assert!(
        !stdout.contains("Current categories"),
        "add query mode must not print a current-categories header, got: {stdout}"
    );
    assert!(
        !xdg_config.join("musu/apps.toml").exists(),
        "add query mode must not register anything"
    );
}

#[test]
fn test_update_unknown_category_warns_on_stderr_and_keeps_custom_value() {
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let xdg_config = temp.path().join("xdg-config");
    let xdg_data = temp.path().join("xdg-data");
    fs::create_dir_all(&home).unwrap();
    let target = temp.path().join("gameapp");
    executable(&target);

    let registry_path = xdg_config.join("musu/apps.toml");
    let manager = Manager::new(
        registry_path.clone(),
        xdg_data.clone(),
        home.join(".local/bin"),
    );
    let entry = manager
        .add(AddOptions {
            target,
            name: Some("gameapp".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let output = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["update", "gameapp", "--categories", "juegoz"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "unknown categories are a warning, not an error; stderr: {stderr}"
    );
    assert!(
        !stdout.contains("Unknown category"),
        "the warning must go to stderr to keep stdout script-friendly; stdout: {stdout}"
    );
    assert!(
        stderr.contains("Unknown category 'juegoz' — kept as custom. Did you mean: Game?"),
        "expected a did-you-mean warning, got: {stderr}"
    );
    assert!(
        stderr.contains("Run 'musu update <app> --categories'"),
        "the hint must point at the no-value query mode, got: {stderr}"
    );
    assert!(
        !stderr.contains("musu categories"),
        "the hint must not reference the removed subcommand, got: {stderr}"
    );
    let updated = manager.get(&entry.id).unwrap();
    assert_eq!(
        updated.categories,
        Some(vec!["juegoz".to_string()]),
        "the unknown value must still be kept as a custom category"
    );

    let known = run_musu(
        &home,
        &xdg_config,
        &xdg_data,
        &["update", "gameapp", "--categories", "Video,dev"],
    );
    let known_stderr = String::from_utf8_lossy(&known.stderr);
    assert!(known.status.success());
    assert!(
        !known_stderr.contains("Unknown category"),
        "known categories must not warn, got: {known_stderr}"
    );
    let updated = manager.get(&entry.id).unwrap();
    assert_eq!(
        updated.categories,
        Some(vec!["Video".to_string(), "Development".to_string()])
    );
}

#[test]
fn test_list_hides_disabled_symlinks() {
    let temp = tempdir().unwrap();
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    let script = temp.path().join("app-without-symlink");
    fs::write(&script, "#!/bin/sh\nexit 0").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
    }

    // Register with no symlink
    manager
        .add(musu::manager::AddOptions {
            target: script.clone(),
            name: Some("NoSymlinkApp".into()),
            no_symlink: true,
            ..Default::default()
        })
        .unwrap();

    // Default list shows symlink=disabled
    let default_output = manager
        .list_with_options(musu::manager::ListOptions {
            hide_disabled_symlinks: false,
            ..Default::default()
        })
        .unwrap();
    assert!(
        default_output.contains("symlink=") && default_output.contains("disabled"),
        "expected symlink and disabled in default list, got: {default_output}"
    );

    // List with hide_disabled_symlinks: true must omit symlink=disabled entirely
    let hidden_output = manager
        .list_with_options(musu::manager::ListOptions {
            hide_disabled_symlinks: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        !hidden_output.contains("symlink="),
        "expected symlink= to be hidden when disabled, got: {hidden_output}"
    );
}

#[test]
fn test_display_setting_hide_disabled_symlinks() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("configured-tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[display]\nhide_disabled_symlinks = true\n",
    )
    .unwrap();
    manager
        .add(AddOptions {
            target,
            name: Some("NoSymlinkTool".into()),
            no_symlink: true,
            ..Default::default()
        })
        .unwrap();

    let output = manager.list().unwrap();
    assert!(
        !output.contains("symlink="),
        "symlink column must be hidden when disabled by config, got: {output}"
    );
}

#[test]
fn test_display_setting_hide_target_and_desktop_ok() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[display]\nhide_target_ok = true\nhide_desktop_ok = true\n",
    )
    .unwrap();
    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyTool".into()),
            ..Default::default()
        })
        .unwrap();

    let output = manager.list().unwrap();
    assert!(
        !output.contains("target="),
        "target=ok must be hidden when hide_target_ok is enabled in config, got: {output}"
    );
    assert!(
        !output.contains("desktop="),
        "desktop=ok must be hidden when hide_desktop_ok is enabled in config, got: {output}"
    );

    // If target becomes missing/broken, it must still be shown despite the config
    fs::remove_file(&target).unwrap();
    let broken_output = manager.list().unwrap();
    assert!(
        broken_output.contains("target="),
        "target must be shown when missing/broken even if hide_target_ok is enabled, got: {broken_output}"
    );
}

#[test]
fn test_display_setting_hide_ok_status_unified_key() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("unified-tool");
    executable(&target);
    let manager = manager_with_settings(&temp);
    fs::create_dir_all(temp.path().join("config")).unwrap();
    fs::write(
        temp.path().join("config/config.toml"),
        "[display]\nhide_ok_status = true\n",
    )
    .unwrap();
    manager
        .add(AddOptions {
            target,
            name: Some("UnifiedTool".into()),
            ..Default::default()
        })
        .unwrap();

    let output = manager.list().unwrap();
    assert!(
        !output.contains("target="),
        "target=ok must be hidden when hide_ok_status is enabled in config, got: {output}"
    );
    assert!(
        !output.contains("desktop="),
        "desktop=ok must be hidden when hide_ok_status is enabled in config, got: {output}"
    );
}

#[test]
fn test_list_cli_hides_target_and_desktop_ok() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("cli-tool");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    manager
        .add(AddOptions {
            target,
            name: Some("CliTool".into()),
            ..Default::default()
        })
        .unwrap();

    let output_all_hidden = manager
        .list_with_options(musu::manager::ListOptions {
            hide_ok: true,
            ..Default::default()
        })
        .unwrap();
    assert!(!output_all_hidden.contains("target="));
    assert!(!output_all_hidden.contains("desktop="));

    let output_only_target = manager
        .list_with_options(musu::manager::ListOptions {
            hide_target_ok: true,
            ..Default::default()
        })
        .unwrap();
    assert!(!output_only_target.contains("target="));
    assert!(output_only_target.contains("desktop="));
}

#[test]
fn test_cli_parses_hide_ok_flags() {
    use clap::Parser;
    let cli =
        musu::cli::Cli::try_parse_from(["musu", "list", "--hide-target-ok", "--hide-desktop-ok"])
            .expect("CLI must parse hide-target-ok and hide-desktop-ok flags");
    match cli.command {
        musu::cli::Command::List(args) => {
            assert!(args.hide_target_ok);
            assert!(args.hide_desktop_ok);
        }
        _ => panic!("expected List command"),
    }

    let cli_unified = musu::cli::Cli::try_parse_from(["musu", "list", "--hide-ok"])
        .expect("CLI must parse --hide-ok flag");
    match cli_unified.command {
        musu::cli::Command::List(args) => {
            assert!(args.hide_ok);
        }
        _ => panic!("expected List command"),
    }
}

#[test]
fn test_update_preserves_existing_icon_when_no_new_icon_provided() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let icon = temp.path().join("icon.png");
    fs::write(&icon, b"valid-icon-data").unwrap();
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            icon: Some(icon),
            name: Some("MyApp".into()),
            ..Default::default()
        })
        .unwrap();

    let icon_path = entry.icon_path.unwrap();
    assert_eq!(fs::read(&icon_path).unwrap(), b"valid-icon-data");

    // Updating target without specifying a new icon
    let new_target = temp.path().join("my-app-v2");
    executable(&new_target);
    manager
        .update(
            &entry.id,
            UpdateOptions {
                target: Some(new_target),
                ..Default::default()
            },
        )
        .unwrap();

    let updated_entry = manager.get(&entry.id).unwrap();
    let updated_icon_path = updated_entry.icon_path.unwrap();
    assert_eq!(
        fs::read(&updated_icon_path).unwrap(),
        b"valid-icon-data",
        "existing icon must not be truncated or corrupted when updating"
    );
}

#[test]
fn test_list_simple_format_has_one_leading_space_indentation() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("indented-app");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    manager
        .add(AddOptions {
            target,
            name: Some("IndentedApp".into()),
            ..Default::default()
        })
        .unwrap();

    let output_no_icons = manager
        .list_with_options(musu::manager::ListOptions {
            no_icons: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        output_no_icons.starts_with(" IndentedApp")
            && !output_no_icons.starts_with("  IndentedApp"),
        "expected list lines to start with exactly 1 leading space, got: {output_no_icons:?}"
    );

    let output_default = manager.list().unwrap();
    assert!(
        output_default.starts_with(" "),
        "expected list lines to start with at least 1 leading space, got: {output_default:?}"
    );
}

#[test]
fn test_add_and_show_with_description() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            description: Some("A wonderful test application".into()),
            ..Default::default()
        })
        .unwrap();

    assert_eq!(
        entry.description,
        Some("A wonderful test application".into())
    );

    // Desktop file must contain Comment=...
    let desktop_path = entry.desktop_file.as_ref().unwrap();
    let desktop_content = fs::read_to_string(desktop_path).unwrap();
    assert!(
        desktop_content.contains("Comment=A wonderful test application\n"),
        "expected Comment in desktop file, got:\n{desktop_content}"
    );

    // show command must format Description
    let show_output = manager.show(&entry.id, ShowOptions::default()).unwrap();
    assert!(
        show_output.contains("Description: A wonderful test application"),
        "expected show to display Description, got:\n{show_output}"
    );

    // Update description
    manager
        .update(
            &entry.id,
            UpdateOptions {
                description: Some("An updated description".into()),
                ..Default::default()
            },
        )
        .unwrap();

    let updated = manager.get(&entry.id).unwrap();
    assert_eq!(updated.description, Some("An updated description".into()));
    let updated_desktop_content = fs::read_to_string(desktop_path).unwrap();
    assert!(
        updated_desktop_content.contains("Comment=An updated description\n"),
        "expected updated Comment in desktop file, got:\n{updated_desktop_content}"
    );
}

#[test]
fn test_list_with_description_flag_and_config() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            description: Some("Audio recorder".into()),
            ..Default::default()
        })
        .unwrap();

    // Default: does not show description
    let output_default = manager.list_with_options(Default::default()).unwrap();
    assert!(
        !output_default.contains("Audio recorder"),
        "expected default list not to contain description, got: {output_default}"
    );

    // With description: true
    let output_with_desc = manager
        .list_with_options(musu::manager::ListOptions {
            description: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        output_with_desc.contains("Audio recorder"),
        "expected list with description: true to contain description, got: {output_with_desc}"
    );

    // Persistent config: [display] description = true
    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        "[display]\ndescription = true\n",
    )
    .unwrap();

    let output_from_config = manager.list_with_options(Default::default()).unwrap();
    assert!(
        output_from_config.contains("Audio recorder"),
        "expected config [display] description = true to show description, got: {output_from_config}"
    );

    // Override with no_description: true
    let output_suppressed = manager
        .list_with_options(musu::manager::ListOptions {
            no_description: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        !output_suppressed.contains("Audio recorder"),
        "expected no_description to suppress description, got: {output_suppressed}"
    );
}

#[test]
fn test_list_table_format_with_description() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            description: Some("Audio recorder".into()),
            ..Default::default()
        })
        .unwrap();

    let output_table = manager
        .list_with_options(musu::manager::ListOptions {
            table: true,
            description: true,
            ..Default::default()
        })
        .unwrap();

    assert!(output_table.contains("DESCRIPTION"));
    assert!(output_table.contains("Audio recorder"));
}

#[test]
fn test_desktop_sanitizes_newlines_and_injections() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("share"),
        temp.path().join("bin"),
    );

    let malicious_name = "LegitApp\nExec=/usr/bin/malicious_payload\nComment=Injected";
    let malicious_comment = "Normal comment\nTerminal=true\nIcon=malicious";
    let malicious_category = "AudioVideo\nCategories=Injected";

    let entry = manager
        .add(AddOptions {
            target,
            name: Some(malicious_name.into()),
            description: Some(malicious_comment.into()),
            categories: Some(vec![malicious_category.into()]),
            ..Default::default()
        })
        .unwrap();

    let desktop_file = entry.desktop_file.expect("desktop file must be created");
    let content = fs::read_to_string(&desktop_file).unwrap();

    // Verify no injected raw lines exist in .desktop file
    for line in content.lines() {
        assert_ne!(
            line, "Exec=/usr/bin/malicious_payload",
            "Injected Exec line must not exist in .desktop file"
        );
        assert_ne!(
            line, "Comment=Injected",
            "Injected Comment line must not exist in .desktop file"
        );
        assert_ne!(
            line, "Categories=Injected",
            "Injected Categories line must not exist in .desktop file"
        );
    }
}
