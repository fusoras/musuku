use musu::cli::{Cli, Command};
use musu::config::{Settings, WatchConfig};
use musu::matcher::{find_icon, normalize_name};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_normalize_name_removes_version_and_architecture_suffixes() {
    assert_eq!(normalize_name("Cool-App-1.2.3-x86_64.AppImage"), "cool-app");
    assert_eq!(normalize_name("tool_v2-amd64"), "tool");
}

#[test]
fn test_find_icon_matches_normalized_stem_case_insensitively() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("Cool-App.PNG"), b"icon").unwrap();
    fs::write(temp.path().join("other.txt"), b"not an icon").unwrap();

    assert_eq!(
        find_icon(temp.path(), "cool-app-1.2.AppImage").unwrap(),
        Some(temp.path().join("Cool-App.PNG"))
    );
}

#[test]
fn test_watch_config_uses_safe_defaults() {
    let settings: Settings = toml::from_str("[watch]\n").unwrap();
    assert_eq!(settings.watch, WatchConfig::default());
    assert!(settings.watch.clean_on_delete);
    assert_eq!(settings.watch.debounce_ms, 1000);
}

#[test]
fn test_watch_cli_parses_scan_only_and_directory_options() {
    let cli = <Cli as clap::Parser>::parse_from([
        "musu",
        "watch",
        "/tmp/apps",
        "--icons-dir",
        "/tmp/icons",
        "--scan-only",
        "--recursive",
        "--dry-run",
    ]);
    let Command::Watch(args) = cli.command else {
        panic!("expected watch command");
    };
    assert_eq!(args.directories, vec![PathBuf::from("/tmp/apps")]);
    assert_eq!(args.icons_dir, Some(PathBuf::from("/tmp/icons")));
    assert!(args.scan_only && args.recursive && args.dry_run);
}

#[test]
fn test_watch_scan_directories_finds_and_registers_appimage() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();
    let appimage_path = app_dir.join("MyTool-1.0.0-x86_64.AppImage");
    fs::write(&appimage_path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&appimage_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&appimage_path, perms).unwrap();
    }

    let manager = musu::manager::Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    let count =
        musu::watcher::scan_directories(&manager, &[app_dir], None, false, false, false).unwrap();

    assert_eq!(count, 1);
    let app = manager.get("mytool").unwrap();
    assert_eq!(app.name, "mytool");
    assert_eq!(app.id, "mytool");
}

#[test]
fn test_watch_without_directories_or_config_errors() {
    let temp = tempdir().unwrap();
    let manager = musu::manager::Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    let args = musu::cli::Watch {
        directories: vec![],
        icons_dir: None,
        continuous: false,
        scan_only: true,
        recursive: false,
        dry_run: false,
        service: None,
    };

    let result = musu::watcher::run(args, manager);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("no watch directories configured or provided"),
        "unexpected error: {err}"
    );
}

#[test]
fn test_extract_version_and_compare() {
    use musu::matcher::{compare_versions, extract_version};
    use std::cmp::Ordering;

    assert_eq!(
        extract_version("sonora-v0.38.0-x86_64.AppImage"),
        Some("0.38.0".to_string())
    );
    assert_eq!(
        extract_version("sonora-v0.42.1-x86_64.AppImage"),
        Some("0.42.1".to_string())
    );
    assert_eq!(compare_versions("0.42.1", "0.38.0"), Ordering::Greater);
    assert_eq!(compare_versions("0.38.0", "0.42.1"), Ordering::Less);
    assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
}

#[test]
fn test_sync_cleans_deleted_appimage_from_registry() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();
    let appimage_path = app_dir.join("PepoMote-1.0-x86_64.AppImage");
    fs::write(&appimage_path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&appimage_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&appimage_path, perms).unwrap();
    }

    let manager = musu::manager::Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    // Initial registration
    manager
        .add(musu::manager::AddOptions {
            target: appimage_path.clone(),
            name: Some("pepomote".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert!(manager.get("pepomote").is_ok());

    // Delete the file from the watched directory
    fs::remove_file(&appimage_path).unwrap();

    // Live sync should detect file removal and clean it up automatically
    let report = musu::watcher::sync_directories(
        &manager,
        &[app_dir],
        None,
        false,
        false,
        true, // clean_on_delete
        false,
    )
    .unwrap();

    assert_eq!(report.removed, 1);
    assert!(
        manager.get("pepomote").is_err(),
        "deleted app must be removed from registry"
    );
}

#[test]
fn test_sync_auto_upgrades_version_when_newer_appimage_detected() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();

    let old_appimage = app_dir.join("sonora-v0.38.0-x86_64.AppImage");
    fs::write(&old_appimage, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&old_appimage).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&old_appimage, perms).unwrap();
    }

    let manager = musu::manager::Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    // Register old version
    manager
        .add(musu::manager::AddOptions {
            target: old_appimage.clone(),
            name: Some("sonora".to_string()),
            ..Default::default()
        })
        .unwrap();

    // Now remove old version and introduce new version
    fs::remove_file(&old_appimage).unwrap();
    let new_appimage = app_dir.join("sonora-v0.42.1-x86_64.AppImage");
    fs::write(&new_appimage, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_appimage).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_appimage, perms).unwrap();
    }

    // Sync
    let report =
        musu::watcher::sync_directories(&manager, &[app_dir], None, false, false, true, false)
            .unwrap();

    assert_eq!(
        report.updated, 1,
        "should have updated 1 app to newer version"
    );
    assert_eq!(report.removed, 0, "should not have deleted the app");
    assert_eq!(report.added, 0, "should not have added a duplicate app");

    let app = manager.get("sonora").unwrap();
    let target_canonical = fs::canonicalize(&new_appimage).unwrap();
    assert_eq!(app.target_bin, target_canonical);
}

#[test]
fn test_list_automatically_syncs_configured_watch_directories() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();

    let old_appimage = app_dir.join("sonora-v0.38.0-x86_64.AppImage");
    fs::write(&old_appimage, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&old_appimage).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&old_appimage, perms).unwrap();
    }

    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // Write config.toml with watch directory
    let toml_content = format!(
        "[watch]\ndirectories = [\"{}\"]\nclean_on_delete = true\n",
        app_dir.display()
    );
    fs::write(config_dir.join("config.toml"), toml_content).unwrap();

    let manager = musu::manager::Manager::new(
        config_dir.join("apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    // Register old version
    manager
        .add(musu::manager::AddOptions {
            target: old_appimage.clone(),
            name: Some("sonora".to_string()),
            ..Default::default()
        })
        .unwrap();

    // Now delete old version and introduce new version
    fs::remove_file(&old_appimage).unwrap();
    let new_appimage = app_dir.join("sonora-v0.42.1-x86_64.AppImage");
    fs::write(&new_appimage, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_appimage).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_appimage, perms).unwrap();
    }

    // Call list_with_options: it should auto-sync and update the target to v0.42.1
    let list_output = manager
        .list_with_options(musu::manager::ListOptions::default())
        .unwrap();

    assert!(list_output.contains("sonora"));
    assert!(list_output.contains("ok"));

    let app = manager.get("sonora").unwrap();
    assert_eq!(app.target_bin, fs::canonicalize(&new_appimage).unwrap());
}

#[test]
fn test_auto_sync_skips_when_directories_unchanged() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();
    let appimage_path = app_dir.join("Tool-1.0-x86_64.AppImage");
    fs::write(&appimage_path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&appimage_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&appimage_path, perms).unwrap();
    }

    let config_dir = temp.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();
    let toml_content = format!(
        "[watch]\ndirectories = [\"{}\"]\nauto_sync = true\n",
        app_dir.display()
    );
    fs::write(config_dir.join("config.toml"), toml_content).unwrap();

    let manager = musu::manager::Manager::new(
        config_dir.join("apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    // First auto_sync: syncs and registers Tool
    let first = musu::watcher::auto_sync_if_configured(&manager).unwrap();
    assert!(first.is_some(), "expected first auto-sync to execute");

    // Second auto_sync immediately without changing anything: must return None (fast-path skip)
    let second = musu::watcher::auto_sync_if_configured(&manager).unwrap();
    assert!(
        second.is_none(),
        "expected unchanged directory to skip auto-sync on fast path"
    );
}

#[test]
fn test_watch_ignores_temporary_downloads_and_empty_files() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("apps");
    fs::create_dir_all(&app_dir).unwrap();

    // 1. Partial Firefox download
    fs::write(app_dir.join("Browser-1.0.AppImage.part"), b"downloading...").unwrap();
    // 2. Chromium download
    fs::write(
        app_dir.join("Editor.AppImage.crdownload"),
        b"downloading...",
    )
    .unwrap();
    // 3. Hidden AppImage
    fs::write(app_dir.join(".hidden.AppImage"), b"hidden").unwrap();
    // 4. Temporary swap file
    fs::write(app_dir.join("Tool.AppImage.tmp"), b"temp").unwrap();
    // 5. Zero-byte incomplete AppImage
    let empty_app = app_dir.join("Empty-App.AppImage");
    fs::write(&empty_app, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&empty_app).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&empty_app, perms).unwrap();
    }

    // 6. Valid AppImage
    let valid_app = app_dir.join("ValidApp-1.0.AppImage");
    fs::write(&valid_app, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&valid_app).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&valid_app, perms).unwrap();
    }

    let manager = musu::manager::Manager::new(
        temp.path().join("config/apps.toml"),
        temp.path().join("data"),
        temp.path().join("bin"),
    );

    let count =
        musu::watcher::scan_directories(&manager, &[app_dir], None, false, false, false).unwrap();

    assert_eq!(
        count, 1,
        "Only valid, non-empty, non-temporary AppImages should be registered"
    );
    let app = manager.get("validapp").unwrap();
    assert_eq!(app.name, "validapp");
    assert!(
        manager.get("empty-app").is_err(),
        "Empty app must not be registered"
    );
}
