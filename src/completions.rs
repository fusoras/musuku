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
        Shell::Zsh => home.join(".local/share/zsh/site-functions/_musu"),
        Shell::Bash => home.join(".local/share/bash-completion/completions/musu"),
        Shell::Fish => home.join(".config/fish/completions/musu.fish"),
        Shell::Elvish => home.join(".config/elvish/lib/musu.elv"),
        Shell::PowerShell => home.join(".config/powershell/musu.ps1"),
        _ => home.join(".local/share/bash-completion/completions/musu"),
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
    clap_complete::generate(shell, &mut cmd, "musu", &mut buf);
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

    if content.contains("_musu") {
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

    let line = "\n# musu completions\n[ -s \"$HOME/.local/share/zsh/site-functions/_musu\" ] && source \"$HOME/.local/share/zsh/site-functions/_musu\"\n";
    file.write_all(line.as_bytes())
        .with_context(|| format!("failed to write to {}", rc_path.display()))?;

    Ok(true)
}

/// Generates the Carapace YAML spec for musu.
pub fn generate_carapace_spec() -> String {
    let mut spec = String::from(
        r#"name: musu
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

  - name: show
    description: Show detailed information and paths for a registered application
    flags:
      --desktop: Print only the desktop file path
      --bin: Print only the target executable path
      --icon: Print only the icon path
      --symlink: Print only the terminal symlink path
      --json: Print the application entry as JSON
    completion:
      positional:
        - ["$(musu list | awk '{print $2}' | tr -d '()')"]

  - name: update
    description: Update metadata, target executable, icon, or categories of a registered application
    flags:
      --target=: New path to the executable file
      --icon=: New path to the icon file
      --name=: New display name
      --categories=: New desktop categories separated by commas
    completion:
      positional:
        - ["$(musu list | awk '{print $2}' | tr -d '()')"]
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
        - ["$(musu list | awk '{print $2}' | tr -d '()')"]

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

/// Installs the Carapace YAML spec for musu into ~/.config/carapace/specs/musu.yaml.
pub fn install_carapace_spec(home: &Path) -> Result<PathBuf> {
    let spec_dir = home.join(".config/carapace/specs");
    fs::create_dir_all(&spec_dir).with_context(|| {
        format!(
            "failed to create carapace specs dir: {}",
            spec_dir.display()
        )
    })?;
    let target = spec_dir.join("musu.yaml");
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
        let spec_file = home.join(".config/carapace/specs/musu.yaml");
        if !spec_file.exists() {
            let _ = install_carapace_spec(&home);
        }
    }

    Ok(installed_path)
}

/// Checks if a directory is present in a list of paths.
pub fn is_dir_in_paths(dir: &Path, paths: &[PathBuf]) -> bool {
    let canonical_dir = dir.canonicalize().ok();
    for p in paths {
        if p == dir {
            return true;
        }
        if let (Some(c_dir), Ok(c_p)) = (&canonical_dir, p.canonicalize())
            && c_dir == &c_p
        {
            return true;
        }
    }
    false
}

/// Checks if a directory is in the system PATH environment variable.
pub fn is_dir_in_path_env(dir: &Path) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        let paths: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
        return is_dir_in_paths(dir, &paths);
    }
    false
}

/// Formats a Bun-style manual completion source line notification for the given shell.
pub fn format_manual_completion_notice(shell: Shell) -> String {
    match shell {
        Shell::Zsh => {
            "Manually add the completion to ~/.zshrc (or similar):\n  [ -s \"$HOME/.local/share/zsh/site-functions/_musu\" ] && source \"$HOME/.local/share/zsh/site-functions/_musu\"".to_string()
        }
        Shell::Bash => {
            "Manually add the completion to ~/.bashrc (or similar):\n  [ -s \"$HOME/.local/share/bash-completion/completions/musu\" ] && source \"$HOME/.local/share/bash-completion/completions/musu\"".to_string()
        }
        _ => String::new(),
    }
}

/// Formats the Bun-style installation success message, including manual PATH configuration if needed.
pub fn format_install_notice(
    bin_path: &Path,
    shell: Shell,
    in_path: bool,
    home_override: Option<&Path>,
) -> String {
    let user_home = dirs::home_dir();
    let home = home_override.or(user_home.as_deref());

    let display_path = if let Some(h) = home {
        if let Ok(rel) = bin_path.strip_prefix(h) {
            format!("~/{}", rel.display())
        } else {
            bin_path.display().to_string()
        }
    } else {
        bin_path.display().to_string()
    };

    let bin_dir = bin_path.parent().unwrap_or(bin_path);
    let display_dir = if let Some(h) = home {
        if let Ok(rel) = bin_dir.strip_prefix(h) {
            format!("$HOME/{}", rel.display())
        } else {
            bin_dir.display().to_string()
        }
    } else {
        bin_dir.display().to_string()
    };

    let rc_file = match shell {
        Shell::Zsh => "~/.zshrc",
        Shell::Fish => "~/.config/fish/config.fish",
        Shell::Bash => "~/.bashrc",
        _ => "~/.profile",
    };

    let mut out = String::new();
    out.push_str(
        "######################################################################## 100.0%\n",
    );
    out.push_str(&format!(
        "musu was installed successfully to {}\n\n",
        display_path
    ));

    if !in_path {
        out.push_str(&format!(
            "Manually add the directory to {} (or similar):\n",
            rc_file
        ));
        if shell == Shell::Fish {
            out.push_str(&format!("  fish_add_path \"{}\"\n\n", display_dir));
        } else {
            out.push_str(&format!("  export PATH=\"{}:$PATH\"\n\n", display_dir));
        }
    }

    out.push_str("To get started, run:\n\n  musu --help\n");
    out
}
