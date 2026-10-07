//! Terminal symlink installation.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn default_base_dir() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".local/bin"))
        .unwrap_or_else(|| PathBuf::from(".local/bin"))
}

pub fn install(
    base_dir: &Path,
    target: &Path,
    name: &str,
    wrapper: Option<&str>,
) -> Result<PathBuf> {
    fs::create_dir_all(base_dir)
        .with_context(|| format!("failed to create {}", base_dir.display()))?;
    let link = base_dir.join(name);
    if link.exists() || fs::symlink_metadata(&link).is_ok() {
        remove(&link)?;
    }
    if writable(base_dir) {
        if let Some(cmd) = wrapper {
            let script = format!("#!/bin/sh\nexec {} \"$@\"\n", cmd);
            fs::write(&link, script)?;
            let mut perms = fs::metadata(&link)?.permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
            fs::set_permissions(&link, perms)?;
        } else {
            std::os::unix::fs::symlink(target, &link)
                .with_context(|| format!("failed to create {}", link.display()))?;
        }
    } else if outside_home(base_dir) {
        if let Some(cmd) = wrapper {
            let script = format!("#!/bin/sh\nexec {} \"$@\"\n", cmd);
            let temp = std::env::temp_dir().join(name);
            fs::write(&temp, script)?;
            run_sudo(&[
                "install",
                "-m",
                "755",
                &temp.to_string_lossy(),
                &link.to_string_lossy(),
            ])?;
            fs::remove_file(temp).ok();
        } else {
            run_sudo(&[
                "ln",
                "-sf",
                &target.to_string_lossy(),
                &link.to_string_lossy(),
            ])?;
        }
    } else {
        return Err(anyhow!(
            "destination is not writable: {}",
            base_dir.display()
        ));
    }
    Ok(link)
}

pub fn remove(link: &Path) -> Result<()> {
    if !link.exists() && fs::symlink_metadata(link).is_err() {
        return Ok(());
    }
    let parent = link.parent().unwrap_or_else(|| Path::new("."));
    if writable(parent) {
        fs::remove_file(link).with_context(|| format!("failed to remove {}", link.display()))?;
    } else if outside_home(parent) {
        run_sudo(&["rm", "-f", &link.to_string_lossy()])?;
    } else {
        return Err(anyhow!("destination is not writable: {}", parent.display()));
    }
    Ok(())
}

fn writable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o222 != 0)
        .unwrap_or(false)
}

fn outside_home(path: &Path) -> bool {
    dirs::home_dir()
        .map(|home| !path.starts_with(home))
        .unwrap_or(true)
}

fn run_sudo(args: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .args(args)
        .status()
        .context("failed to invoke sudo")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("sudo command failed with status {status}"))
    }
}

use std::os::unix::fs::PermissionsExt;
