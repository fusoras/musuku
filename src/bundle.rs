//! Portable bundle metadata (`.musu.toml`).
//!
//! Stores application metadata (name, relative binary path, icon, categories,
//! and description) directly alongside the application in its parent directory.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const BUNDLE_CONFIG_NAME: &str = ".musu.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BundleMetadata {
    pub app: BundleApp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BundleApp {
    pub name: String,
    pub bin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descriptions: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
}

/// Find `.musu.toml` either directly inside `target` (if it's a directory)
/// or in the parent directory of `target` (if it's a file).
pub fn find_bundle_file(target: &Path) -> Option<PathBuf> {
    if target.is_dir() {
        let bundle_file = target.join(BUNDLE_CONFIG_NAME);
        if bundle_file.is_file() {
            return Some(bundle_file);
        }
    }
    if let Some(parent) = target.parent() {
        let bundle_file = parent.join(BUNDLE_CONFIG_NAME);
        if bundle_file.is_file() {
            return Some(bundle_file);
        }
    }
    None
}

/// Load bundle metadata from a `.musu.toml` file.
pub fn load_bundle(file_path: &Path) -> Result<BundleMetadata> {
    let contents = fs::read_to_string(file_path)
        .with_context(|| format!("failed to read bundle file {}", file_path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("failed to parse bundle file {}", file_path.display()))
}

/// Save bundle metadata to a `.musu.toml` file in `dir`.
pub fn save_bundle(dir: &Path, metadata: &BundleMetadata) -> Result<PathBuf> {
    let path = dir.join(BUNDLE_CONFIG_NAME);
    let contents =
        toml::to_string_pretty(metadata).context("failed to serialize bundle metadata to TOML")?;
    fs::write(&path, contents)
        .with_context(|| format!("failed to write bundle file {}", path.display()))?;
    Ok(path)
}

/// Format a path relative to `base_dir` if inside it, or with `~/` if inside `home_dir`,
/// or as an absolute path.
pub fn serialize_bundle_path(path: &Path, base_dir: &Path, home_dir: Option<&Path>) -> String {
    // Check if path is inside base_dir
    if let Ok(relative) = path.strip_prefix(base_dir) {
        return relative.to_string_lossy().into_owned();
    }
    // Check if path is inside home_dir
    if let Some(home) = home_dir
        && let Ok(relative) = path.strip_prefix(home)
    {
        return format!("~/{}", relative.to_string_lossy());
    }
    path.to_string_lossy().into_owned()
}

/// Resolve a bundle path string (which may be relative, `~/...`, or absolute)
/// back into an absolute `PathBuf` relative to `base_dir`.
pub fn resolve_bundle_path(path_str: &str, base_dir: &Path, home_dir: Option<&Path>) -> PathBuf {
    if path_str.starts_with("~/") || path_str == "~" {
        if let Some(home) = home_dir {
            let without_tilde = path_str.strip_prefix("~/").unwrap_or("");
            return home.join(without_tilde);
        } else if let Some(home) = dirs::home_dir() {
            let without_tilde = path_str.strip_prefix("~/").unwrap_or("");
            return home.join(without_tilde);
        }
    }
    let p = Path::new(path_str);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base_dir.join(p)
    }
}
