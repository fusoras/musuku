//! Systemd user service and SysVinit service definitions for `musu watch`.

use std::path::{Path, PathBuf};

/// Generates the systemd user service unit definition.
pub fn generate_systemd_unit(bin_path: &Path) -> String {
    format!(
        r#"[Unit]
Description=Musu AppImage Folder Watcher & Desktop Integrator
Documentation=https://github.com/fusoras/musu
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

/// Resolves the current musu binary path.
pub fn resolve_musu_bin() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(canonical) = std::fs::canonicalize(&exe) {
            return canonical;
        }
        return exe;
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join("musu");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin/musu");
        if cargo_bin.is_file() {
            return cargo_bin;
        }
        let local_bin = home.join(".local/bin/musu");
        if local_bin.is_file() {
            return local_bin;
        }
    }
    PathBuf::from("musu")
}

/// Silently ensures the user systemd service file exists on disk at the specified path.
/// If it does not exist, generates it and writes it without printing to stdout or stderr.
pub fn ensure_systemd_unit_exists_at(target_path: &Path, bin_path: &Path) {
    if !target_path.exists() {
        if let Some(parent) = target_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = generate_systemd_unit(bin_path);
        let _ = std::fs::write(target_path, content);
    }
}

/// Silently ensures the user systemd service file exists in ~/.config/systemd/user/.
pub fn ensure_systemd_unit_exists() {
    let target = systemd_unit_path();
    let bin = resolve_musu_bin();
    ensure_systemd_unit_exists_at(&target, &bin);
}
