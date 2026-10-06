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

/// Generates the Carapace YAML spec for geico.
pub fn generate_carapace_spec() -> String {
    let mut spec = String::from(
        r#"name: geico
description: Integrate portable applications into the desktop
commands:
  - name: add
    description: Register an executable and integrate it into the desktop and PATH
    flags:
      --name=: Display name of the application
      --icon=: Path to the icon file
      --categories=: Desktop categories separated by commas
      --bin-name=: Custom command name for the terminal symlink
      --dest-bin=: Custom destination directory for the symlink
      --no-desktop: Skip creating a .desktop launcher entry
      --no-symlink: Skip creating a terminal $PATH symlink
    completion:
      positional:
        - ["$files"]
      flag:
        icon: ["$files"]
        dest-bin: ["$dirs"]
        categories:
"#,
    );

    for cat in crate::categories::STANDARD_CATEGORIES {
        spec.push_str(&format!(
            "          - \"{}\\t{}\"\n",
            cat.name, cat.description
        ));
    }

    spec.push_str(
        r#"
  - name: list
    description: List all managed applications with health checks

  - name: update
    description: Update metadata, target executable, icon, or categories of a registered application
    flags:
      --target=: New path to the executable file
      --icon=: New path to the icon file
      --name=: New display name
      --categories=: New desktop categories separated by commas
    completion:
      positional:
        - ["$(geico list | awk '{print $2}' | tr -d '()')"]
      flag:
        target: ["$files"]
        icon: ["$files"]
        categories:
"#,
    );

    for cat in crate::categories::STANDARD_CATEGORIES {
        spec.push_str(&format!(
            "          - \"{}\\t{}\"\n",
            cat.name, cat.description
        ));
    }

    spec.push_str(
        r#"
  - name: remove
    description: Safely remove desktop launcher and symlink
    completion:
      positional:
        - ["$(geico list | awk '{print $2}' | tr -d '()')"]

  - name: categories
    description: List standard FreeDesktop application categories with descriptions and examples

  - name: completions
    description: Generate shell completion script
    flags:
      --install: Install completion script automatically into user shell directory
      --stdout: Print raw completion script to stdout instead of installing to disk
    completion:
      positional:
        - ["bash", "elvish", "fish", "powershell", "zsh"]
"#,
    );

    spec
}

/// Installs the Carapace YAML spec for geico into ~/.config/carapace/specs/geico.yaml.
pub fn install_carapace_spec(home: &Path) -> Result<PathBuf> {
    let spec_dir = home.join(".config/carapace/specs");
    fs::create_dir_all(&spec_dir).with_context(|| {
        format!(
            "failed to create carapace specs dir: {}",
            spec_dir.display()
        )
    })?;
    let target = spec_dir.join("geico.yaml");
    let content = generate_carapace_spec();
    fs::write(&target, content)
        .with_context(|| format!("failed to write carapace spec: {}", target.display()))?;
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
    let mut installed_path = None;
    if !path.exists() {
        installed_path = Some(install_completions_to(shell, &home)?);
    }

    // If Carapace is configured on the user system, also ensure Carapace spec is installed
    let carapace_config = home.join(".config/carapace");
    if carapace_config.exists() {
        let spec_file = carapace_config.join("specs/geico.yaml");
        if !spec_file.exists() {
            let _ = install_carapace_spec(&home);
        }
    }

    Ok(installed_path)
}
