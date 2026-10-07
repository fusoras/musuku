use geico::cli::{Cli, Command};
use geico::config::{Settings, WatchConfig};
use geico::matcher::{find_icon, normalize_name};
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
        "geico",
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
