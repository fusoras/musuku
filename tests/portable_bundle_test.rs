use musu::manager::{AddOptions, Manager, UpdateOptions};
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

#[test]
fn test_add_with_portable_creates_musu_toml() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);

    let app_dir = temp.path().join("my-custom-app");
    fs::create_dir_all(&app_dir).unwrap();

    let bin_path = app_dir.join("launch.sh");
    executable(&bin_path);

    let icon_path = app_dir.join("icon.png");
    fs::write(&icon_path, "fake-png").unwrap();

    manager
        .add(AddOptions {
            target: bin_path.clone(),
            name: Some("My Custom App".to_string()),
            icon: Some(icon_path.clone()),
            description: Some("Awesome tool".to_string()),
            categories: Some(vec!["Utility".to_string(), "Development".to_string()]),
            portable: true,
            ..AddOptions::default()
        })
        .unwrap();

    let musu_toml = app_dir.join(".musu.toml");
    assert!(
        musu_toml.exists(),
        ".musu.toml must be created when portable=true"
    );

    let content = fs::read_to_string(&musu_toml).unwrap();
    assert!(content.contains("name = \"My Custom App\""));
    assert!(content.contains("bin = \"launch.sh\""));
    assert!(content.contains("icon = \"icon.png\""));
    assert!(content.contains("description = \"Awesome tool\""));
    assert!(content.contains("Development"));
}

#[test]
fn test_add_autodetects_existing_musu_toml_without_flags() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);

    let app_dir = temp.path().join("shared-app");
    fs::create_dir_all(&app_dir).unwrap();

    let bin_path = app_dir.join("run.sh");
    executable(&bin_path);

    let icon_path = app_dir.join("assets").join("logo.png");
    fs::create_dir_all(icon_path.parent().unwrap()).unwrap();
    fs::write(&icon_path, "fake-png").unwrap();

    let musu_toml_content = r#"
[app]
name = "Shared App"
bin = "run.sh"
icon = "assets/logo.png"
description = "Imported from bundle"
categories = ["Game"]
"#;
    fs::write(app_dir.join(".musu.toml"), musu_toml_content).unwrap();

    // Add pointing to bin_path (or app_dir) without explicit name/icon/description
    let entry = manager
        .add(AddOptions {
            target: bin_path.clone(),
            ..AddOptions::default()
        })
        .unwrap();

    assert_eq!(entry.name, "Shared App");
    assert_eq!(entry.description.as_deref(), Some("Imported from bundle"));
    assert_eq!(entry.categories.as_deref(), Some(&["Game".to_string()][..]));
    assert!(entry.icon_path.is_some());
}

#[test]
fn test_portable_icon_outside_bundle_normalizes_to_tilde_or_absolute() {
    let temp = tempdir().unwrap();
    let fake_home = temp.path().join("home_user");
    fs::create_dir_all(&fake_home).unwrap();

    let manager = manager_with(&temp).with_home(fake_home.clone());

    let app_dir = fake_home.join("apps").join("portable-tool");
    fs::create_dir_all(&app_dir).unwrap();
    let bin_path = app_dir.join("tool.sh");
    executable(&bin_path);

    // Icon in ~/icons/tool.png (outside app_dir, inside HOME)
    let home_icons = fake_home.join("icons");
    fs::create_dir_all(&home_icons).unwrap();
    let home_icon = home_icons.join("tool.png");
    fs::write(&home_icon, "fake-icon").unwrap();

    manager
        .add(AddOptions {
            target: bin_path.clone(),
            icon: Some(home_icon),
            portable: true,
            ..AddOptions::default()
        })
        .unwrap();

    let musu_toml = app_dir.join(".musu.toml");
    assert!(musu_toml.exists());
    let content = fs::read_to_string(&musu_toml).unwrap();
    assert!(content.contains("icon = \"~/icons/tool.png\""));
}

#[test]
fn test_update_with_portable_creates_or_updates_musu_toml() {
    let temp = tempdir().unwrap();
    let manager = manager_with(&temp);

    let app_dir = temp.path().join("update-app");
    fs::create_dir_all(&app_dir).unwrap();
    let bin_path = app_dir.join("main.sh");
    executable(&bin_path);

    let entry = manager
        .add(AddOptions {
            target: bin_path.clone(),
            name: Some("Initial Name".to_string()),
            ..AddOptions::default()
        })
        .unwrap();

    let musu_toml = app_dir.join(".musu.toml");
    assert!(!musu_toml.exists());

    // Update with portable = true and new name/description
    manager
        .update(
            &entry.id,
            UpdateOptions {
                name: Some("Updated Name".to_string()),
                description: Some("New description".to_string()),
                portable: true,
                ..UpdateOptions::default()
            },
        )
        .unwrap();

    assert!(musu_toml.exists());
    let content = fs::read_to_string(&musu_toml).unwrap();
    assert!(content.contains("name = \"Updated Name\""));
    assert!(content.contains("description = \"New description\""));
    assert!(content.contains("bin = \"main.sh\""));
}
