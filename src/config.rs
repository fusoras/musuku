//! Persistence for the application registry (`apps.toml`).
//!
//! The default location is `~/.config/geico/apps.toml`, but every function
//! accepts an explicit path and [`config_path`] accepts a custom base
//! directory, so tests can operate on isolated `tempfile` workspaces without
//! ever touching the real user configuration.

use crate::models::AppRegistry;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Default base directory for geico state: `$XDG_CONFIG_HOME/geico`
/// (i.e. `~/.config/geico` on a standard setup).
pub fn default_base_dir() -> PathBuf {
    let config_dir =
        dirs::config_dir().or_else(|| dirs::home_dir().map(|home| home.join(".config")));
    match config_dir {
        Some(dir) => dir.join("geico"),
        None => PathBuf::from(".config/geico"),
    }
}

/// Resolve the `apps.toml` path inside an arbitrary base directory.
///
/// This is the isolation seam used by tests: pass a `tempdir()` path instead
/// of [`default_base_dir`].
pub fn config_path(base_dir: &Path) -> PathBuf {
    base_dir.join("apps.toml")
}

/// Resolve the optional program settings path alongside `apps.toml`.
pub fn settings_path(base_dir: &Path) -> PathBuf {
    base_dir.join("config.toml")
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_symlink_appimages", alias = "appimage_symlinks")]
    pub symlink_appimages: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            symlink_appimages: default_symlink_appimages(),
        }
    }
}

fn default_symlink_appimages() -> bool {
    true
}

/// Load optional program settings, using safe defaults when the file is absent.
pub fn load_settings(base_dir: &Path) -> Result<Settings> {
    let path = settings_path(base_dir);
    if !path.exists() {
        return Ok(Settings::default());
    }
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read settings file {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse settings file {}", path.display()))
}

/// The real user config path: `~/.config/geico/apps.toml`.
pub fn default_config_path() -> PathBuf {
    config_path(&default_base_dir())
}

/// Load the registry from `path`.
///
/// A missing file is not an error: it yields an empty registry (first run).
/// Malformed TOML is reported as an error instead of silently discarding it.
pub fn load(path: &Path) -> Result<AppRegistry> {
    if !path.exists() {
        return Ok(AppRegistry::new());
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    let registry: AppRegistry = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config file {}", path.display()))?;
    Ok(registry)
}

/// Save `registry` to `path`, creating missing parent directories as needed.
pub fn save(path: &Path, registry: &AppRegistry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    let contents =
        toml::to_string_pretty(registry).context("failed to serialize registry to TOML")?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write config file {}", path.display()))?;
    Ok(())
}
