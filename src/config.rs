//! Persistence for the application registry (`apps.toml`).
//!
//! The default location is `~/.config/musu/apps.toml`, but every function
//! accepts an explicit path and [`config_path`] accepts a custom base
//! directory, so tests can operate on isolated `tempfile` workspaces without
//! ever touching the real user configuration.

use crate::models::AppRegistry;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default base directory for musu state: `$XDG_CONFIG_HOME/musu`
/// (i.e. `~/.config/musu` on a standard setup).
pub fn default_base_dir() -> PathBuf {
    dirs::config_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
        .map(|dir| dir.join("musu"))
        .unwrap_or_else(|| PathBuf::from(".config/musu"))
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub defaults: Defaults,
    #[serde(default)]
    pub display: Display,
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub sandbox: SandboxConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_sandbox_backend")]
    pub backend: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: default_sandbox_backend(),
        }
    }
}

fn default_sandbox_backend() -> String {
    "auto".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WatchConfig {
    #[serde(default)]
    pub directories: Vec<PathBuf>,
    #[serde(default)]
    pub icons_dir: Option<PathBuf>,
    #[serde(default)]
    pub auto_chmod: bool,
    #[serde(default)]
    pub recursive: bool,
    #[serde(default = "default_clean_on_delete")]
    pub clean_on_delete: bool,
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
    #[serde(default = "default_auto_sync")]
    pub auto_sync: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            directories: Vec::new(),
            icons_dir: None,
            auto_chmod: false,
            recursive: false,
            clean_on_delete: true,
            debounce_ms: 1000,
            auto_sync: true,
        }
    }
}

fn default_clean_on_delete() -> bool {
    true
}

fn default_auto_sync() -> bool {
    true
}

fn default_debounce_ms() -> u64 {
    1000
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Defaults {
    #[serde(default = "default_symlink_appimages", alias = "appimage_symlinks")]
    pub symlink_appimages: bool,
    #[serde(default = "default_categories")]
    pub categories: Vec<String>,
    #[serde(default = "default_auto_extract")]
    pub auto_extract: bool,
    #[serde(default = "default_update_desktop_db")]
    pub update_desktop_db: bool,
    #[serde(default = "default_dest_bin")]
    pub dest_bin: PathBuf,
    #[serde(default)]
    pub portable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Display {
    #[serde(default, alias = "images")]
    pub icons: IconPolicy,
    #[serde(default)]
    pub list_icon_size: ListIconSize,
    #[serde(default)]
    pub show_icon_size: ShowIconSize,
    #[serde(default)]
    pub paths: PathPolicy,
    #[serde(default)]
    pub color: ColorPolicy,
    #[serde(default)]
    pub list_format: ListFormat,
    #[serde(default)]
    pub hide_disabled_symlinks: bool,
    #[serde(default)]
    pub hide_target_ok: bool,
    #[serde(default)]
    pub hide_desktop_ok: bool,
    #[serde(
        default,
        alias = "hide_ok",
        alias = "hide_target_desktop_ok",
        alias = "hide_healthy"
    )]
    pub hide_ok_status: bool,
    #[serde(
        default,
        alias = "show_description",
        alias = "show_descriptions",
        alias = "descriptions"
    )]
    pub description: bool,
}

pub type ImagePolicy = IconPolicy;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IconPolicy {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListIconSize {
    #[default]
    Compact,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ShowIconSize {
    #[default]
    Standard,
    Compact,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PathPolicy {
    #[default]
    Tilde,
    Absolute,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColorPolicy {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListFormat {
    #[default]
    Simple,
    Table,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            symlink_appimages: default_symlink_appimages(),
            categories: default_categories(),
            auto_extract: default_auto_extract(),
            update_desktop_db: default_update_desktop_db(),
            dest_bin: default_dest_bin(),
            portable: false,
        }
    }
}

fn default_symlink_appimages() -> bool {
    false
}

fn default_categories() -> Vec<String> {
    vec!["Utility".to_string()]
}

fn default_auto_extract() -> bool {
    true
}

fn default_update_desktop_db() -> bool {
    true
}

fn default_dest_bin() -> PathBuf {
    PathBuf::from("~/.local/bin")
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

/// Save optional program settings to `config.toml` in `base_dir`.
pub fn save_settings(base_dir: &Path, settings: &Settings) -> Result<()> {
    let path = settings_path(base_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create settings dir {}", parent.display()))?;
    }
    let contents =
        toml::to_string_pretty(settings).context("failed to serialize settings to TOML")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write settings file {}", path.display()))?;
    Ok(())
}

/// The real user config path: `~/.config/musu/apps.toml`.
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
