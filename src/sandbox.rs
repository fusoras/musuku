use crate::config::{self, SandboxPolicy, Settings};
use std::path::Path;

/// Resolve the effective sandbox backend for `settings`.
///
/// `"auto"` probes for an installed `bwrap` first and falls back to
/// `firejail`; `"none"` (or neither installed) means no sandboxing.
pub fn resolve_backend(settings: &Settings) -> String {
    if settings.sandbox.backend == "auto" {
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
    }
}

/// The fully effective policy for an app: the global `[sandbox.policy]`
/// (with defaults filled) merged with the optional per-app policy.
pub fn effective_settings_policy(
    settings: &Settings,
    app: Option<&SandboxPolicy>,
) -> SandboxPolicy {
    config::effective_policy(&settings.sandbox.global_effective(), app)
}

/// Advisory warnings for policy restrictions the given backend cannot
/// enforce. Empty for bwrap (which supports the full policy) and for
/// policies with nothing unsupported. Never fatal.
pub fn validate_policy(backend: &str, policy: &SandboxPolicy) -> Vec<String> {
    if backend != "firejail" {
        return Vec::new();
    }
    let mut warnings = Vec::new();
    if policy.home_writable == Some(false) {
        warnings.push(
            "sandbox: backend \"firejail\" does not support home_writable=false; this restriction will be ignored"
                .to_string(),
        );
    }
    if policy
        .extra_rw
        .as_ref()
        .is_some_and(|paths| !paths.is_empty())
    {
        warnings.push(
            "sandbox: backend \"firejail\" does not support extra_rw; these paths will be ignored"
                .to_string(),
        );
    }
    warnings
}

/// Expand a leading `~` (alone or as `~/...`) to the real home directory
/// via `dirs::home_dir()`; other paths are returned unchanged. `None` when
/// a `~` expansion is required but no home directory is known.
fn expand_tilde(raw: &str) -> Option<String> {
    if raw == "~" {
        return dirs::home_dir().map(|home| home.display().to_string());
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        let home = dirs::home_dir()?;
        return Some(format!("{}/{}", home.display(), rest));
    }
    Some(raw.to_string())
}

/// The two directories that are always hidden inside the sandbox,
/// regardless of any policy.
fn always_hidden() -> Vec<String> {
    dirs::home_dir()
        .map(|home| {
            let home_str = home.display().to_string();
            vec![format!("{home_str}/.ssh"), format!("{home_str}/.gnupg")]
        })
        .unwrap_or_default()
}

/// Whether the sandbox effectively applies to `target_bin`.
///
/// The global `[sandbox].enabled` flag only wraps AppImages: scripts and
/// plain binaries are never sandboxed implicitly. An explicit per-app
/// `entry_sandbox` flag always wins, in both directions — `--sandbox`
/// force-wraps any executable, `--no-sandbox` opts an AppImage out.
pub fn sandbox_effective_enabled(
    settings: &Settings,
    entry_sandbox: Option<bool>,
    target_bin: &Path,
) -> bool {
    match entry_sandbox {
        Some(explicit) => explicit,
        None => {
            settings.sandbox.enabled && crate::extractor::is_appimage(target_bin).unwrap_or(false)
        }
    }
}

/// Generate the command prefix wrapping `target_bin` in the desktop file
/// and symlink, honoring the effective sandbox policy (global merged with
/// the optional per-app `entry_policy`). With no policy defined the output
/// is byte-for-byte identical to the pre-policy behavior.
pub fn generate_sandbox_exec(
    settings: &Settings,
    entry_sandbox: Option<bool>,
    entry_policy: Option<&SandboxPolicy>,
    target_bin: &Path,
) -> String {
    let enabled = entry_sandbox.unwrap_or(settings.sandbox.enabled);
    let target = target_bin.display().to_string();

    if !enabled {
        return target;
    }

    let backend = resolve_backend(settings);
    let policy = effective_settings_policy(settings, entry_policy);
    let network = policy.network.unwrap_or(true);
    let home_writable = policy.home_writable.unwrap_or(true);
    let hidden: Vec<String> = policy
        .hide_paths
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|path| expand_tilde(path))
        .collect();
    let extra_rw: Vec<String> = policy
        .extra_rw
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|path| expand_tilde(path))
        .collect();
    let extra_ro: Vec<String> = policy
        .extra_ro
        .as_deref()
        .unwrap_or_default()
        .iter()
        .filter_map(|path| expand_tilde(path))
        .collect();
    let protected = always_hidden();

    match backend.as_str() {
        "bwrap" => {
            // "Grants display (Wayland / X11), audio (PulseAudio / PipeWire), and GPU access while isolating sensitive directories (~/.ssh, ~/.gnupg)."
            let mut cmd = vec!["bwrap".to_string()];
            cmd.push("--unshare-all".to_string());
            if network {
                cmd.push("--share-net".to_string());
            }
            cmd.push("--ro-bind / /".to_string());
            cmd.push("--dev /dev".to_string());
            cmd.push("--proc /proc".to_string());
            cmd.push("--bind /tmp /tmp".to_string());
            if let Some(home) = dirs::home_dir() {
                let home_str = home.display().to_string();
                if home_writable {
                    cmd.push(format!("--bind {} {}", home_str, home_str));
                } else {
                    cmd.push(format!("--ro-bind {} {}", home_str, home_str));
                }
                cmd.push(format!("--tmpfs {}/.ssh", home_str));
                cmd.push(format!("--tmpfs {}/.gnupg", home_str));
            }
            for path in &hidden {
                // ~/.ssh and ~/.gnupg are already covered above.
                if protected.contains(path) {
                    continue;
                }
                cmd.push(format!("--tmpfs {path}"));
            }
            for path in &extra_rw {
                // The protected directories are never exposed, no matter
                // what the policy asks for.
                if protected.contains(path) {
                    continue;
                }
                cmd.push(format!("--bind {path} {path}"));
            }
            for path in &extra_ro {
                if protected.contains(path) {
                    continue;
                }
                cmd.push(format!("--ro-bind {path} {path}"));
            }
            cmd.push(target);
            cmd.join(" ")
        }
        "firejail" => {
            let mut cmd = vec!["firejail".to_string()];
            cmd.push("--noprofile".to_string());
            if network {
                cmd.push("--net=host".to_string());
            } else {
                cmd.push("--net=none".to_string());
            }
            cmd.push("--env=WAYLAND_DISPLAY".to_string());
            cmd.push("--env=DISPLAY".to_string());
            cmd.push("--env=XDG_RUNTIME_DIR".to_string());
            cmd.push("--env=PULSE_SERVER".to_string());
            cmd.push("--env=PIPEWIRE_REMOTE".to_string());
            cmd.push("--blacklist=~/.ssh".to_string());
            cmd.push("--blacklist=~/.gnupg".to_string());
            for path in &hidden {
                if protected.contains(path) {
                    continue;
                }
                cmd.push(format!("--blacklist={path}"));
            }
            // `home_writable=false` and `extra_rw` are not reliably
            // supported by firejail; see `validate_policy`.
            for path in &extra_ro {
                if protected.contains(path) {
                    continue;
                }
                cmd.push(format!("--blacklist={path}"));
            }
            cmd.push(target);
            cmd.join(" ")
        }
        _ => {
            // fallback if none is installed or unknown
            target
        }
    }
}
