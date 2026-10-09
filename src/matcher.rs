//! Matching AppImage names with icons in a dedicated directory.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const ICON_EXTENSIONS: &[&str] = &["png", "svg", "webp", "xpm", "ico"];

/// Returns true if the path has a supported icon extension.
pub fn is_icon_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ICON_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        })
}

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

/// Extract version string from an application filename if present.
pub fn extract_version(value: &str) -> Option<String> {
    let stem = Path::new(value)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(value);
    let mut parts: Vec<&str> = stem.split(['-', '_']).collect();
    let mut found_version = None;
    while parts.len() > 1 {
        let last = parts.last().copied().unwrap_or_default();
        let is_version = last.chars().all(|c| c.is_ascii_digit() || c == '.')
            || (last.starts_with(['v', 'V'])
                && last[1..].chars().all(|c| c.is_ascii_digit() || c == '.')
                && !last[1..].is_empty());
        let is_architecture = matches!(
            last,
            "x86_64" | "x86" | "amd64" | "aarch64" | "arm64" | "armv7" | "i686"
        );
        if is_version {
            let v = if last.starts_with(['v', 'V']) {
                &last[1..]
            } else {
                last
            };
            found_version = Some(v.to_string());
            parts.pop();
        } else if is_architecture {
            parts.pop();
        } else {
            break;
        }
    }
    found_version
}

/// Compare two version strings segment by segment (numerically where possible).
pub fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    let clean1 = v1.trim_start_matches(['v', 'V']);
    let clean2 = v2.trim_start_matches(['v', 'V']);

    let segs1: Vec<&str> = clean1.split('.').collect();
    let segs2: Vec<&str> = clean2.split('.').collect();

    let max_len = segs1.len().max(segs2.len());
    for i in 0..max_len {
        let s1 = segs1.get(i).copied().unwrap_or("0");
        let s2 = segs2.get(i).copied().unwrap_or("0");

        match (s1.parse::<u64>(), s2.parse::<u64>()) {
            (Ok(n1), Ok(n2)) => {
                if n1 != n2 {
                    return n1.cmp(&n2);
                }
            }
            _ => {
                let cmp = s1.cmp(s2);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
    std::cmp::Ordering::Equal
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
        if !path.is_file() || !is_icon_file(&path) {
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

/// Known relative paths to probe for icons near an executable or game bundle.
pub const COMMON_ICON_RELATIVE_PATHS: &[&str] = &[
    // Ren'Py standard & common variations
    "game/gui/window_icon.png",
    "game/window_icon.png",
    "game/gui/icon.png",
    "game/icon.png",
    "gui/window_icon.png",
    // Zen Browser / Firefox / Gecko builds (prioritizing higher resolution)
    "browser/chrome/icons/default/default128.png",
    "browser/chrome/icons/default/default64.png",
    "browser/chrome/icons/default/default48.png",
    "browser/chrome/icons/default/default32.png",
    "browser/chrome/icons/default/default16.png",
    "chrome/icons/default/default128.png",
    "chrome/icons/default/default64.png",
    "chrome/icons/default/default48.png",
    "chrome/icons/default/default32.png",
    "chrome/icons/default/default16.png",
    // Godot / Love2D / Common Root
    "icon.png",
    "icon.svg",
    "logo.png",
    "logo.svg",
    "app.png",
    "app.svg",
    // RPG Maker / Engines
    "icon/icon.png",
    "www/icon/icon.png",
    // Common subdirectories
    "assets/icon.png",
    "assets/icon.svg",
    "assets/icons/icon.png",
    "resources/icon.png",
    "resources/app.png",
    "icons/icon.png",
];

/// Probes the parent directory of an executable for standard icon locations.
///
/// Looks for:
/// 1. Known relative paths (Ren'Py `game/gui/window_icon.png`, Godot `icon.png`, etc.)
/// 2. Executable name matches in the parent folder (e.g. `<stem>.png`, `<stem>.svg`)
/// 3. Unity Player icons (e.g. `<stem>_Data/Resources/UnityPlayer.png`)
pub fn probe_app_icon(target_bin: &Path) -> Option<PathBuf> {
    let parent = target_bin.parent()?;

    // 1. Probe specific known relative paths
    for rel_path in COMMON_ICON_RELATIVE_PATHS {
        let candidate = parent.join(rel_path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 2. Binary stem checks (e.g. `MyGame.png` or `MyGame_Data/Resources/UnityPlayer.png`)
    if let Some(stem) = target_bin.file_stem().and_then(|s| s.to_str()) {
        for ext in ICON_EXTENSIONS {
            let candidate = parent.join(format!("{stem}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // Unity Linux player icon convention
        let unity_candidate = parent
            .join(format!("{stem}_Data"))
            .join("Resources")
            .join("UnityPlayer.png");
        if unity_candidate.is_file() {
            return Some(unity_candidate);
        }
    }

    None
}

/// Returns true if the target executable appears to belong to a Ren'Py game.
///
/// Ren'Py games typically contain:
/// 1. A `renpy` directory alongside the executable, and a `game` directory.
/// 2. Or a `game` directory containing `gui/window_icon.png`, `.rpyc` scripts, or `.rpa` archives.
pub fn is_renpy_game(target_bin: &Path) -> bool {
    let Some(parent) = target_bin.parent() else {
        return false;
    };

    let renpy_dir = parent.join("renpy");
    let game_dir = parent.join("game");

    if renpy_dir.is_dir() && game_dir.is_dir() {
        return true;
    }

    if game_dir.is_dir() {
        if game_dir.join("gui/window_icon.png").is_file()
            || game_dir.join("window_icon.png").is_file()
        {
            return true;
        }

        if let Ok(entries) = fs::read_dir(&game_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str())
                    && (ext.eq_ignore_ascii_case("rpyc") || ext.eq_ignore_ascii_case("rpa"))
                {
                    return true;
                }
            }
        }
    }

    false
}

/// Compute Levenshtein distance between two strings
#[allow(clippy::needless_range_loop)]
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

    for i in 0..=a_chars.len() {
        dp[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        dp[0][j] = j;
    }

    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a_chars.len()][b_chars.len()]
}

/// Suggest a similar app using normalized Levenshtein edit distance.
pub fn suggest_similar_app(query: &str, candidates: &[&str]) -> Option<String> {
    let mut best_match = None;
    let mut best_dist = usize::MAX;

    let query_lower = query.to_lowercase();

    for &cand in candidates {
        // We compare the raw candidate and the normalized candidate.
        let cand_lower = cand.to_lowercase();
        let norm_cand = normalize_name(&cand_lower);

        for c in [&cand_lower, &norm_cand] {
            let dist = levenshtein(&query_lower, c);
            let max_len = query_lower.chars().count().max(c.chars().count());
            let score = if max_len == 0 {
                0.0
            } else {
                dist as f64 / max_len as f64
            };

            if (dist <= 2 || score < 0.4) && dist < best_dist {
                best_dist = dist;
                best_match = Some(cand.to_string());
            }
        }
    }

    best_match
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggest_similar_app() {
        let candidates = vec!["shotcut", "firefox", "gimp", "vlc"];
        assert_eq!(
            suggest_similar_app("shotct", &candidates),
            Some("shotcut".to_string())
        );
        assert_eq!(
            suggest_similar_app("gim", &candidates),
            Some("gimp".to_string())
        );
        assert_eq!(suggest_similar_app("asdfghj", &candidates), None);
    }
}
