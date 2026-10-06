//! Automatic shell completion discovery and installation.

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::Shell;
use std::fs::{self, OpenOptions};
use std::io::Write;
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

/// Checks if carapace binary is present in any of the specified directories.
pub fn is_carapace_in_dirs(dirs: &[&Path]) -> bool {
    for dir in dirs {
        let candidate = dir.join("carapace");
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Checks whether Carapace is installed and executable in system PATH.
pub fn is_carapace_installed() -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        let paths: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
        let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
        return is_carapace_in_dirs(&path_refs);
    }
    false
}

/// Finds the user's active .zshrc file, respecting $ZDOTDIR, ~/.config/zsh, and ~/.zshrc.
pub fn find_zshrc_path(home: &Path) -> PathBuf {
    if let Ok(zdotdir) = std::env::var("ZDOTDIR") {
        let candidate = PathBuf::from(zdotdir).join(".zshrc");
        if candidate.exists() {
            return candidate;
        }
    }
    let config_zshrc = home.join(".config/zsh/.zshrc");
    if config_zshrc.exists() {
        return config_zshrc;
    }
    home.join(".zshrc")
}

/// Ensures the Bun-style source line exists in the user's .zshrc.
/// Returns Ok(true) if the line was newly appended, Ok(false) if it already existed.
pub fn ensure_zshrc_source_line(rc_path: &Path) -> Result<bool> {
    let content = if rc_path.exists() {
        fs::read_to_string(rc_path)
            .with_context(|| format!("failed to read {}", rc_path.display()))?
    } else {
        String::new()
    };

    if content.contains("_geico") {
        return Ok(false);
    }

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(rc_path)
        .with_context(|| format!("failed to open {} for appending", rc_path.display()))?;

    let line = "\n# geico completions\n[ -s \"$HOME/.local/share/zsh/site-functions/_geico\" ] && source \"$HOME/.local/share/zsh/site-functions/_geico\"\n";
    file.write_all(line.as_bytes())
        .with_context(|| format!("failed to write to {}", rc_path.display()))?;

    Ok(true)
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
/// - If shell is Zsh, writes the completion script and adds the Bun-style source line to .zshrc.
/// - Only installs the Carapace spec if Carapace is actually installed on the system.
///
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

    // Bun-style direct source line for Zsh to guarantee zero-friction loading without $fpath changes
    if shell == Shell::Zsh {
        let zshrc = find_zshrc_path(&home);
        let _ = ensure_zshrc_source_line(&zshrc);
    }

    // Only install Carapace spec if Carapace binary is actually installed in PATH
    if is_carapace_installed() {
        let spec_file = home.join(".config/carapace/specs/geico.yaml");
        if !spec_file.exists() {
            let _ = install_carapace_spec(&home);
        }
    }

    Ok(installed_path)
}
