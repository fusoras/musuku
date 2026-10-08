//! Core data model for Milestone 1: [`AppEntry`] (a registered portable
//! executable) and [`AppRegistry`] (the auditable collection of entries that
//! backs `apps.toml`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single application registered with `musu`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppEntry {
    /// Slugified identifier derived from the display name (e.g. `my-cool-app`).
    pub id: String,
    /// Display name; falls back to the target binary filename when not given.
    pub name: String,
    /// Path to the original executable / AppImage. Never modified or deleted.
    pub target_bin: PathBuf,
    /// Path of the generated `.desktop` launcher, if any.
    pub desktop_file: Option<PathBuf>,
    /// Path of the generated `$PATH` symlink, if any.
    pub symlink: Option<PathBuf>,
    /// Path of the registered icon, if any.
    pub icon_path: Option<PathBuf>,
    /// Desktop categories (e.g. `["Development", "Utility"]`), if any.
    pub categories: Option<Vec<String>>,
    /// Application description or tooltip, if any.
    #[serde(default)]
    pub description: Option<String>,
    /// Whether this app runs sandboxed.
    #[serde(default)]
    pub sandbox: Option<bool>,
    /// Registration timestamp.
    pub created_at: DateTime<Utc>,
}

impl AppEntry {
    /// Build a new entry for `target_bin`.
    ///
    /// When `name` is `None` the display name is derived from the binary
    /// filename (e.g. `/path/to/blender-4.2/blender` -> `blender` or
    /// `Firefox.AppImage` -> `Firefox`), and the `id` is the slug of the
    /// resulting name.
    pub fn new(name: Option<&str>, target_bin: PathBuf) -> Self {
        let name = name
            .map(str::to_string)
            .unwrap_or_else(|| fallback_name(&target_bin));
        let id = slugify(&name);
        Self {
            id,
            name,
            target_bin,
            desktop_file: None,
            symlink: None,
            icon_path: None,
            categories: None,
            description: None,
            sandbox: None,
            created_at: Utc::now(),
        }
    }
}

/// Derive the fallback display name from a binary path, stripping a trailing
/// `.appimage` extension case-insensitively.
pub fn fallback_name(target_bin: &Path) -> String {
    let name = target_bin
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let suffix = ".appimage";
    if name.len() >= suffix.len() && name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    {
        name[..name.len() - suffix.len()].to_string()
    } else {
        name
    }
}

/// Slugify `input` into a filesystem/id-friendly token: ASCII lowercase with
/// runs of non-alphanumeric characters collapsed into single `-` separators
/// (no leading or trailing separators).
pub fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut pending_sep = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !slug.is_empty() {
                slug.push('-');
            }
            pending_sep = false;
            slug.push(ch.to_ascii_lowercase());
        } else {
            pending_sep = true;
        }
    }
    slug
}

/// The auditable collection of registered applications, persisted as
/// `~/.config/musu/apps.toml`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AppRegistry {
    /// All managed entries, in registration order.
    pub apps: Vec<AppEntry>,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self { apps: Vec::new() }
    }

    /// Find an entry by id or display name, matched case-insensitively.
    pub fn find(&self, id_or_name: &str) -> Option<&AppEntry> {
        self.apps.iter().find(|app| matches_query(app, id_or_name))
    }

    /// Register a new entry.
    pub fn add(&mut self, entry: AppEntry) {
        self.apps.push(entry);
    }

    /// Mutate the entry matching `id_or_name` in place with `f`.
    ///
    /// Returns `None` when no entry matches.
    pub fn update<F>(&mut self, id_or_name: &str, f: F) -> Option<&mut AppEntry>
    where
        F: FnOnce(&mut AppEntry),
    {
        let index = self
            .apps
            .iter()
            .position(|app| matches_query(app, id_or_name))?;
        f(&mut self.apps[index]);
        Some(&mut self.apps[index])
    }

    /// Remove the entry matching `id_or_name`, returning it if found.
    ///
    /// Only the registry entry is removed; the original binary is never
    /// touched (non-destructive invariant).
    pub fn remove(&mut self, id_or_name: &str) -> Option<AppEntry> {
        let index = self
            .apps
            .iter()
            .position(|app| matches_query(app, id_or_name))?;
        Some(self.apps.remove(index))
    }
}

/// Case-insensitive match of an entry's id or display name against `query`.
fn matches_query(entry: &AppEntry, query: &str) -> bool {
    entry.id.eq_ignore_ascii_case(query) || entry.name.eq_ignore_ascii_case(query)
}
