//! Systemd user service and SysVinit service manager for `musu watch`.

use crate::{cli::WatchServiceCmd, colors, config, manager::Manager};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

/// Generates the systemd user service unit definition.
pub fn generate_systemd_unit(bin_path: &Path) -> String {
    format!(
        r#"[Unit]
Description=Musu AppImage Folder Watcher & Desktop Integrator
Documentation=https://github.com/uruka1/musu
After=default.target

[Service]
Type=simple
ExecStart={} watch --continuous
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
"#,
        bin_path.display()
    )
}

/// Generates the SysV LSB init script.
pub fn generate_sysv_script(bin_path: &Path, user: &str) -> String {
    format!(
        r#"#!/bin/sh
### BEGIN INIT INFO
# Provides:          musu-watch
# Required-Start:    $local_fs $remote_fs
# Required-Stop:     $local_fs $remote_fs
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: Musu AppImage Folder Watcher
# Description:       Musu AppImage Folder Watcher & Desktop Integrator
### END INIT INFO

NAME="musu-watch"
DAEMON="{}"
DAEMON_ARGS="watch --continuous"
USER="{}"
PIDFILE="/var/run/$NAME.pid"

case "$1" in
    start)
        echo "Starting $NAME..."
        start-stop-daemon --start --background --make-pidfile --pidfile "$PIDFILE" \
            --chuid "$USER" --exec "$DAEMON" -- $DAEMON_ARGS
        ;;
    stop)
        echo "Stopping $NAME..."
        start-stop-daemon --stop --pidfile "$PIDFILE" --oknodo
        rm -f "$PIDFILE"
        ;;
    restart)
        $0 stop
        sleep 1
        $0 start
        ;;
    status)
        start-stop-daemon --status --pidfile "$PIDFILE"
        case "$?" in
            0) echo "$NAME is running." ;;
            1) echo "$NAME is dead but pidfile exists." ;;
            3) echo "$NAME is not running." ;;
            *) echo "$NAME status unknown." ;;
        esac
        ;;
    *)
        echo "Usage: $0 {{start|stop|restart|status}}"
        exit 1
        ;;
esac

exit 0
"#,
        bin_path.display(),
        user
    )
}

/// Path to the systemd user service unit.
pub fn systemd_unit_path() -> PathBuf {
    dirs::config_dir()
        .map(|c| c.join("systemd/user/musu-watch.service"))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config/systemd/user/musu-watch.service")
        })
}

/// Path to the SysV init script.
pub fn sysv_script_path() -> PathBuf {
    PathBuf::from("/etc/init.d/musu-watch")
}

/// Dispatches the service CLI subcommand.
pub fn dispatch(cmd: WatchServiceCmd, manager: &Manager) -> Result<()> {
    match cmd {
        WatchServiceCmd::InstallService {
            now,
            sysv,
            directories,
        } => {
            install(now, sysv, &directories, manager)?;
        }
        WatchServiceCmd::UninstallService { sysv } => {
            uninstall(sysv)?;
        }
    }
    Ok(())
}

/// Resolves current musu binary path.
fn resolve_musu_bin() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(canonical) = fs::canonicalize(&exe) {
            return Ok(canonical);
        }
        return Ok(exe);
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("musu");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin/musu");
        if cargo_bin.is_file() {
            return Ok(cargo_bin);
        }
        let local_bin = home.join(".local/bin/musu");
        if local_bin.is_file() {
            return Ok(local_bin);
        }
    }
    bail!("could not determine path to musu executable");
}

/// Installs the background watch service.
pub fn install(now: bool, sysv: bool, directories: &[PathBuf], manager: &Manager) -> Result<()> {
    let base_dir = manager.settings_base().to_path_buf();
    let mut settings = config::load_settings(&base_dir)?;

    // Persist any passed watch directories to config.toml
    if !directories.is_empty() {
        let mut modified = false;
        for dir in directories {
            let resolved = if dir.is_relative() {
                if let Ok(cur) = std::env::current_dir() {
                    cur.join(dir)
                } else {
                    dir.clone()
                }
            } else {
                dir.clone()
            };
            if !settings.watch.directories.contains(&resolved) {
                settings.watch.directories.push(resolved);
                modified = true;
            }
        }
        if modified {
            config::save_settings(&base_dir, &settings)?;
        }
    }

    let bin_path = resolve_musu_bin()?;

    if sysv {
        install_sysv(&bin_path, now)?;
    } else {
        install_systemd(&bin_path, now)?;
    }

    Ok(())
}

fn install_systemd(bin_path: &Path, now: bool) -> Result<()> {
    let unit_file = systemd_unit_path();
    if let Some(parent) = unit_file.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create systemd user directory: {}",
                parent.display()
            )
        })?;
    }

    let unit_content = generate_systemd_unit(bin_path);
    fs::write(&unit_file, unit_content)
        .with_context(|| format!("failed to write systemd unit to: {}", unit_file.display()))?;

    // Reload systemd user daemon
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();

    if now {
        let status = Command::new("systemctl")
            .args(["--user", "enable", "--now", "musu-watch.service"])
            .status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "{}✔ Systemd user service installed and started: {}{}",
                    colors::BOLD_GREEN,
                    unit_file.display(),
                    colors::RESET
                );
            }
            _ => {
                println!(
                    "{}✔ Systemd user service installed: {}{}",
                    colors::BOLD_GREEN,
                    unit_file.display(),
                    colors::RESET
                );
                eprintln!(
                    "{}Note: could not automatically enable systemd service; try manually:{}",
                    colors::BOLD_YELLOW,
                    colors::RESET
                );
                eprintln!("  systemctl --user enable --now musu-watch.service");
            }
        }
    } else {
        println!(
            "{}✔ Systemd user service installed: {}{}",
            colors::BOLD_GREEN,
            unit_file.display(),
            colors::RESET
        );
        println!("  To enable and start: systemctl --user enable --now musu-watch.service");
    }

    println!("\n{}Control commands:{}", colors::BOLD_CYAN, colors::RESET);
    println!("  systemctl --user status musu-watch.service");
    println!("  systemctl --user stop musu-watch.service");
    println!("  journalctl --user -u musu-watch.service -f");

    Ok(())
}

fn install_sysv(bin_path: &Path, now: bool) -> Result<()> {
    let user = std::env::var("USER").unwrap_or_else(|_| "root".into());
    let script_content = generate_sysv_script(bin_path, &user);
    let script_path = sysv_script_path();

    // Check if we can write directly or need sudo
    if fs::write(&script_path, &script_content).is_err() {
        // Escalate via sudo
        let temp_file = std::env::temp_dir().join(format!("musu-watch-{}", std::process::id()));
        fs::write(&temp_file, &script_content)?;

        let status = Command::new("sudo")
            .args([
                "cp",
                &temp_file.to_string_lossy(),
                &script_path.to_string_lossy(),
            ])
            .status()
            .context("failed to copy SysV init script via sudo")?;

        let _ = fs::remove_file(&temp_file);

        if !status.success() {
            bail!("failed to install SysV script to {}", script_path.display());
        }

        let _ = Command::new("sudo")
            .args(["chmod", "+x", &script_path.to_string_lossy()])
            .status();
    } else {
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    println!(
        "{}✔ SysV init script installed: {}{}",
        colors::BOLD_GREEN,
        script_path.display(),
        colors::RESET
    );

    if now {
        let _ = Command::new("sudo")
            .args(["service", "musu-watch", "start"])
            .status();
    }

    println!("\n{}Control commands:{}", colors::BOLD_CYAN, colors::RESET);
    println!("  sudo service musu-watch status");
    println!("  sudo service musu-watch stop");
    println!("  sudo service musu-watch restart");

    Ok(())
}

/// Uninstalls the background watch service.
pub fn uninstall(sysv: bool) -> Result<()> {
    if sysv {
        let script_path = sysv_script_path();
        let _ = Command::new("sudo")
            .args(["service", "musu-watch", "stop"])
            .status();
        if script_path.exists() {
            let _ = Command::new("sudo")
                .args(["rm", "-f", &script_path.to_string_lossy()])
                .status();
        }
        println!(
            "{}✔ SysV init service uninstalled.{}",
            colors::BOLD_GREEN,
            colors::RESET
        );
    } else {
        let unit_file = systemd_unit_path();
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "--now", "musu-watch.service"])
            .status();
        if unit_file.exists() {
            let _ = fs::remove_file(&unit_file);
        }
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        println!(
            "{}✔ Systemd user service uninstalled.{}",
            colors::BOLD_GREEN,
            colors::RESET
        );
    }
    Ok(())
}
