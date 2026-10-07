use crate::config::Settings;
use std::path::Path;

pub fn generate_sandbox_exec(
    settings: &Settings,
    entry_sandbox: Option<bool>,
    target_bin: &Path,
) -> String {
    let enabled = entry_sandbox.unwrap_or(settings.sandbox.enabled);
    let target = target_bin.display().to_string();

    if !enabled {
        return target;
    }

    let backend = if settings.sandbox.backend == "auto" {
        if std::process::Command::new("bwrap")
            .arg("--version")
            .output()
            .is_ok()
        {
            "bwrap".to_string()
        } else if std::process::Command::new("firejail")
            .arg("--version")
            .output()
            .is_ok()
        {
            "firejail".to_string()
        } else {
            "none".to_string()
        }
    } else {
        settings.sandbox.backend.clone()
    };

    match backend.as_str() {
        "bwrap" => {
            // "Grants display (Wayland / X11), audio (PulseAudio / PipeWire), and GPU access while isolating sensitive directories (~/.ssh, ~/.gnupg)."
            let mut cmd = vec!["bwrap".to_string()];
            cmd.push("--unshare-all".to_string());
            cmd.push("--share-net".to_string());
            cmd.push("--ro-bind / /".to_string());
            cmd.push("--dev /dev".to_string());
            cmd.push("--proc /proc".to_string());
            cmd.push("--bind /tmp /tmp".to_string());
            if let Some(home) = dirs::home_dir() {
                let home_str = home.display().to_string();
                cmd.push(format!("--bind {} {}", home_str, home_str));
                cmd.push(format!("--tmpfs {}/.ssh", home_str));
                cmd.push(format!("--tmpfs {}/.gnupg", home_str));
            }
            cmd.push(target);
            cmd.join(" ")
        }
        "firejail" => {
            let mut cmd = vec!["firejail".to_string()];
            cmd.push("--noprofile".to_string());
            cmd.push("--net=host".to_string());
            cmd.push("--env=WAYLAND_DISPLAY".to_string());
            cmd.push("--env=DISPLAY".to_string());
            cmd.push("--env=XDG_RUNTIME_DIR".to_string());
            cmd.push("--env=PULSE_SERVER".to_string());
            cmd.push("--env=PIPEWIRE_REMOTE".to_string());
            cmd.push("--blacklist=~/.ssh".to_string());
            cmd.push("--blacklist=~/.gnupg".to_string());
            cmd.push(target);
            cmd.join(" ")
        }
        _ => {
            // fallback if none is installed or unknown
            target
        }
    }
}
