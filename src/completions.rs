//! Automatic shell completion discovery and installation.

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::Shell;
use std::fs;
use std::path::{Path, PathBuf};

/// Detects the user's active shell from the SHELL environment variable.
pub fn detect_shell(env_shell: Option<&str>) -> Shell {
    let shell_str = env_shell.unwrap_or("");
    let lower = shell_str.to_ascii_lowercase();
    if lower.ends_with("zsh") || lower.contains("/zsh") {
        Shell::Zsh
    } else if lower.ends_with("fish") || lower.contains("/fish") {
        Shell::Fish
    } else if lower.ends_with("bash") || lower.contains("/bash") {
        Shell::Bash
    } else if lower.ends_with("elvish") {
        Shell::Elvish
    } else if lower.ends_with("powershell") || lower.ends_with("pwsh") {
        Shell::PowerShell
    } else {
        Shell::Bash
    }
}

/// Returns the standard user-space completions file path for the given shell.
pub fn get_completion_path(shell: Shell, home: &Path) -> PathBuf {
    match shell {
        Shell::Zsh => home.join(".local/share/zsh/site-functions/_geico"),
        Shell::Bash => home.join(".local/share/bash-completion/completions/geico"),
        Shell::Fish => home.join(".config/fish/completions/geico.fish"),
        Shell::Elvish => home.join(".config/elvish/lib/geico.elv"),
        Shell::PowerShell => home.join(".config/powershell/geico.ps1"),
        _ => home.join(".local/share/bash-completion/completions/geico"),
    }
}

/// Installs the shell completion script for the target shell into the specified home directory.
pub fn install_completions_to(shell: Shell, home: &Path) -> Result<PathBuf> {
    let target = get_completion_path(shell, home);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }
    let mut cmd = crate::cli::Cli::command();
    let mut buf = Vec::new();
    clap_complete::generate(shell, &mut cmd, "geico", &mut buf);
    fs::write(&target, buf)
        .with_context(|| format!("failed to write completion script to: {}", target.display()))?;
    Ok(target)
}

/// Automatically ensures that completions for the active shell are installed.
/// If already installed, does nothing (silent idempotency).
pub fn ensure_completions_installed() -> Result<Option<PathBuf>> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return Ok(None),
    };
    let shell = detect_shell(std::env::var("SHELL").ok().as_deref());
    let path = get_completion_path(shell, &home);
    if !path.exists() {
        let installed = install_completions_to(shell, &home)?;
        Ok(Some(installed))
    } else {
        Ok(None)
    }
}
