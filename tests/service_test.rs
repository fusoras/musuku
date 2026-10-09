use clap::Parser;
use musu::cli::{Cli, Command};
use musu::service::{generate_systemd_unit, generate_sysv_script, systemd_unit_path};
use std::path::{Path, PathBuf};

#[test]
fn test_cli_watch_has_no_service_subcommand() {
    let cli = Cli::try_parse_from(["musu", "watch", "--continuous", "/path/to/apps"])
        .expect("must parse watch args");
    match cli.command {
        Command::Watch(args) => {
            assert!(args.continuous);
            assert_eq!(args.directories, vec![PathBuf::from("/path/to/apps")]);
        }
        other => panic!("expected Command::Watch, got {other:?}"),
    }
}

#[test]
fn test_generate_systemd_unit_content() {
    let bin_path = Path::new("/usr/local/bin/musu");
    let unit = generate_systemd_unit(bin_path);

    assert!(unit.contains("[Unit]"));
    assert!(unit.contains("Description=Musu AppImage Folder Watcher"));
    assert!(unit.contains("ExecStart=/usr/local/bin/musu watch --continuous"));
    assert!(unit.contains("Restart=on-failure"));
    assert!(unit.contains("[Install]"));
    assert!(unit.contains("WantedBy=default.target"));
}

#[test]
fn test_generate_sysv_script_content() {
    let bin_path = Path::new("/usr/local/bin/musu");
    let script = generate_sysv_script(bin_path, "testuser");

    assert!(script.starts_with("#!/bin/sh"));
    assert!(script.contains("DAEMON=\"/usr/local/bin/musu\""));
    assert!(script.contains("DAEMON_ARGS=\"watch --continuous\""));
    assert!(script.contains("USER=\"testuser\""));
    assert!(script.contains("start)"));
    assert!(script.contains("stop)"));
    assert!(script.contains("status)"));
}

#[test]
fn test_systemd_unit_path_is_in_user_config() {
    let path = systemd_unit_path();
    assert!(path.ends_with(".config/systemd/user/musu-watch.service"));
}

#[test]
fn test_ensure_systemd_unit_exists_creates_file_silently() {
    let temp = tempfile::tempdir().unwrap();
    let service_file = temp.path().join("systemd/user/musu-watch.service");
    let bin_path = Path::new("/custom/bin/musu");

    assert!(!service_file.exists());
    musu::service::ensure_systemd_unit_exists_at(&service_file, bin_path);
    assert!(service_file.exists());

    let content = std::fs::read_to_string(&service_file).unwrap();
    assert!(content.contains("ExecStart=/custom/bin/musu watch --continuous"));

    // Calling it again should not overwrite or error
    musu::service::ensure_systemd_unit_exists_at(&service_file, bin_path);
    assert!(service_file.exists());
}
