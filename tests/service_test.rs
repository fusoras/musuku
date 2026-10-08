use clap::Parser;
use musu::cli::{Cli, Command, WatchServiceCmd};
use musu::service::{generate_systemd_unit, generate_sysv_script, systemd_unit_path};
use std::path::{Path, PathBuf};

#[test]
fn test_cli_parses_watch_service_subcommands() {
    // musu watch service install
    let cli =
        Cli::try_parse_from(["musu", "watch", "service", "install"]).expect("must parse install");
    match cli.command {
        Command::Watch(args) => match args.service {
            Some(WatchServiceCmd::Install {
                now,
                sysv,
                directories,
            }) => {
                assert!(now, "default now must be true");
                assert!(!sysv, "default sysv must be false");
                assert!(directories.is_empty());
            }
            other => panic!("expected Install, got {other:?}"),
        },
        other => panic!("expected Command::Watch, got {other:?}"),
    }

    // musu watch service install --now=false --sysv /path/to/apps
    let cli = Cli::try_parse_from([
        "musu",
        "watch",
        "service",
        "install",
        "--now=false",
        "--sysv",
        "/path/to/apps",
    ])
    .expect("must parse install with flags");
    match cli.command {
        Command::Watch(args) => match args.service {
            Some(WatchServiceCmd::Install {
                now,
                sysv,
                directories,
            }) => {
                assert!(!now);
                assert!(sysv);
                assert_eq!(directories, vec![PathBuf::from("/path/to/apps")]);
            }
            other => panic!("expected Install, got {other:?}"),
        },
        other => panic!("expected Command::Watch, got {other:?}"),
    }

    // musu watch service status
    let cli =
        Cli::try_parse_from(["musu", "watch", "service", "status"]).expect("must parse status");
    match cli.command {
        Command::Watch(args) => match args.service {
            Some(WatchServiceCmd::Status { sysv }) => assert!(!sysv),
            other => panic!("expected Status, got {other:?}"),
        },
        other => panic!("expected Command::Watch, got {other:?}"),
    }

    // musu watch service uninstall
    let cli = Cli::try_parse_from(["musu", "watch", "service", "uninstall"])
        .expect("must parse uninstall");
    match cli.command {
        Command::Watch(args) => match args.service {
            Some(WatchServiceCmd::Uninstall { sysv }) => assert!(!sysv),
            other => panic!("expected Uninstall, got {other:?}"),
        },
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
