//! Persistence for the application registry (`apps.toml`).
//!
//! The default location is `~/.config/musu/apps.toml`, but every function
//! accepts an explicit path and [`config_path`] accepts a custom base
//! directory, so tests can operate on isolated `tempfile` workspaces without
//! ever touching the real user configuration.

use crate::models::AppRegistry;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// Profile types musu knows how to classify and honor today. A type in this
/// list may be enabled by its `[sandbox.<type>]` profile; any other profile
/// section is reported by `musu doctor` and ignored by resolution (its apps
/// are sandboxed only through an explicit `--sandbox` flag).
pub const SUPPORTED_SANDBOX_PROFILES: &[&str] = &["appimage"];

/// The `[sandbox]` table: one profile per app type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct SandboxConfig {
    /// Per-type profiles. Flattened so each `[sandbox.<type>]` table maps
    /// onto one profile.
    #[serde(flatten)]
    pub profiles: SandboxProfiles,
}

impl SandboxConfig {
    /// The profile configured for `kind`, if any. Unsupported types never
    /// resolve to a profile, no matter what their section says.
    pub fn profile_for_type(&self, kind: &str) -> Option<&SandboxProfile> {
        if !SUPPORTED_SANDBOX_PROFILES.contains(&kind) {
            return None;
        }
        if kind == "appimage" {
            return self.profiles.appimage.as_ref();
        }
        self.profiles.extra.get(kind)
    }
}

/// The profiles stored under `[sandbox]`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct SandboxProfiles {
    /// The AppImage profile (`[sandbox.appimage]`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appimage: Option<SandboxProfile>,
    /// Profiles for types musu doesn't support yet: kept so unknown sections
    /// parse (and `musu doctor` can warn about them) instead of failing.
    #[serde(flatten)]
    pub extra: BTreeMap<String, SandboxProfile>,
}

impl SandboxProfiles {
    /// Every configured profile as `(type, profile)` pairs, including types
    /// outside [`SUPPORTED_SANDBOX_PROFILES`].
    pub fn iter(&self) -> impl Iterator<Item = (&str, &SandboxProfile)> {
        self.appimage
            .iter()
            .map(|profile| ("appimage", profile))
            .chain(
                self.extra
                    .iter()
                    .map(|(kind, profile)| (kind.as_str(), profile)),
            )
    }
}

/// The sandbox configuration for one app type (`[sandbox.<type>]`): an
/// optional enablement flag, a backend, and the restrictions that apply to
/// every sandboxed app of that type.
///
/// Every restriction field is optional so "unset" can be distinguished from
/// an explicit `false`/empty value. See [`SandboxPolicy`] and
/// [`effective_policy`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SandboxProfile {
    /// `None` = not configured: the sandbox stays off for this type and
    /// `musu doctor` warns about the missing key.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Backend for this type: `auto` | `bwrap` | `firejail` | `none`.
    #[serde(default = "default_sandbox_backend")]
    pub backend: String,
    /// `None` inherits the default; `Some(false)` denies network access.
    #[serde(default)]
    pub network: Option<bool>,
    /// `None` inherits the default; `Some(false)` mounts `$HOME` read-only.
    #[serde(default)]
    pub home_writable: Option<bool>,
    /// Paths to hide entirely (tmpfs over them in bwrap, blacklisted in
    /// firejail). Replaces the default list.
    #[serde(default)]
    pub hide_paths: Option<Vec<String>>,
    /// Extra paths to expose read-write. Replaces the default list.
    #[serde(default)]
    pub extra_rw: Option<Vec<String>>,
    /// Extra paths to expose read-only. Replaces the default list.
    #[serde(default)]
    pub extra_ro: Option<Vec<String>>,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        Self {
            enabled: None,
            backend: default_sandbox_backend(),
            network: None,
            home_writable: None,
            hide_paths: None,
            extra_rw: None,
            extra_ro: None,
        }
    }
}

impl SandboxProfile {
    /// The profile's restrictions as a [`SandboxPolicy`]: unset fields stay
    /// `None` so they inherit the effective defaults (or the per-app
    /// overrides, see [`effective_policy`]).
    pub fn policy(&self) -> SandboxPolicy {
        SandboxPolicy {
            network: self.network,
            home_writable: self.home_writable,
            hide_paths: self.hide_paths.clone(),
            extra_rw: self.extra_rw.clone(),
            extra_ro: self.extra_ro.clone(),
        }
    }
}

fn default_sandbox_backend() -> String {
    "auto".to_string()
}

/// Sandbox restrictions, configurable per app type (`config.toml`, inside
/// the `[sandbox.<type>]` profile) and per app (`apps.toml`).
///
/// Every field is optional so "not defined" can be distinguished from an
/// explicit `false`/empty value. Merging happens field by field via
/// [`effective_policy`]: a `Some` field of the per-app policy replaces the
/// profile value wholesale (list fields are replaced, never concatenated),
/// a `None` field inherits it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct SandboxPolicy {
    /// `None` inherits; `Some(false)` denies network access.
    pub network: Option<bool>,
    /// `None` inherits; `Some(false)` mounts `$HOME` read-only.
    pub home_writable: Option<bool>,
    /// Paths to hide entirely (tmpfs over them in bwrap, blacklisted in
    /// firejail). Replaces the profile list.
    pub hide_paths: Option<Vec<String>>,
    /// Extra paths to expose read-write. Replaces the profile list.
    pub extra_rw: Option<Vec<String>>,
    /// Extra paths to expose read-only. Replaces the profile list.
    pub extra_ro: Option<Vec<String>>,
}

/// Merge a profile policy with an optional per-app policy, field by field:
/// every `Some` field of `app` replaces the profile value, `None` inherits
/// it. Fields unset in both fall back to the effective defaults (network on,
/// home writable, empty path lists), so the returned policy always has every
/// field `Some`.
pub fn effective_policy(profile: &SandboxPolicy, app: Option<&SandboxPolicy>) -> SandboxPolicy {
    let mut merged = profile.clone();
    if let Some(app) = app {
        if app.network.is_some() {
            merged.network = app.network;
        }
        if app.home_writable.is_some() {
            merged.home_writable = app.home_writable;
        }
        if app.hide_paths.is_some() {
            merged.hide_paths = app.hide_paths.clone();
        }
        if app.extra_rw.is_some() {
            merged.extra_rw = app.extra_rw.clone();
        }
        if app.extra_ro.is_some() {
            merged.extra_ro = app.extra_ro.clone();
        }
    }
    merged.network.get_or_insert(true);
    merged.home_writable.get_or_insert(true);
    merged.hide_paths.get_or_insert_with(Vec::new);
    merged.extra_rw.get_or_insert_with(Vec::new);
    merged.extra_ro.get_or_insert_with(Vec::new);
    merged
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
///
/// A config still written in the legacy flat `[sandbox]` format is rejected
/// with a migration hint instead of being silently reinterpreted.
pub fn load_settings(base_dir: &Path) -> Result<Settings> {
    let path = settings_path(base_dir);
    if !path.exists() {
        return Ok(Settings::default());
    }
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read settings file {}", path.display()))?;
    if let Some(key) = legacy_sandbox_key(&contents) {
        bail!(
            "legacy [sandbox] format detected in {} (found `{key}`); migrate to per-type \
             profiles:\n  [sandbox.appimage]\n  enabled = true\n  backend = \"auto\"",
            path.display()
        );
    }
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse settings file {}", path.display()))
}

/// The legacy flat key found directly under `[sandbox]`, if any: the
/// pre-profile format kept `enabled`, `backend`, and a nested
/// `[sandbox.policy]` table there, while the profile format only holds
/// `[sandbox.<type>]` tables. Unparsable TOML yields `None` so the regular
/// parse reports it with its own context.
fn legacy_sandbox_key(contents: &str) -> Option<&'static str> {
    let parsed = toml::from_str::<toml::Value>(contents).ok()?;
    let sandbox = parsed.as_table()?.get("sandbox")?.as_table()?;
    ["enabled", "backend", "policy"]
        .into_iter()
        .find(|key| sandbox.contains_key(*key))
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
