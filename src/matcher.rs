//! Matching AppImage names with icons in a dedicated directory.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const ICON_EXTENSIONS: &[&str] = &["png", "svg", "webp", "xpm", "ico"];

/// Normalize an application filename for comparisons with icon names.
pub fn normalize_name(value: &str) -> String {
    let stem = Path::new(value)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase();
    let mut parts: Vec<&str> = stem.split(['-', '_']).collect();
    while parts.len() > 1 {
        let last = parts.last().copied().unwrap_or_default();
        let is_version = last.chars().all(|c| c.is_ascii_digit() || c == '.')
            || (last.starts_with('v') && last[1..].chars().all(|c| c.is_ascii_digit() || c == '.'));
        let is_architecture = matches!(
            last,
            "x86_64" | "x86" | "amd64" | "aarch64" | "arm64" | "armv7" | "i686"
        );
        if is_version || is_architecture {
            parts.pop();
        } else {
            break;
        }
    }
    parts.join("-")
}

/// Find a supported icon whose normalized stem matches `app_name`.
pub fn find_icon(icons_dir: &Path, app_name: &str) -> Result<Option<PathBuf>> {
    if !icons_dir.is_dir() {
        return Ok(None);
    }
    let wanted = normalize_name(app_name);
    for entry in fs::read_dir(icons_dir)
        .with_context(|| format!("failed to read icons directory {}", icons_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file()
            || !path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ICON_EXTENSIONS
                        .iter()
                        .any(|allowed| ext.eq_ignore_ascii_case(allowed))
                })
        {
            continue;
        }
        if path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| normalize_name(stem) == wanted)
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}
