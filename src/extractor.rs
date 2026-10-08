//! Best-effort metadata and icon extraction for standard Type 2 AppImages.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DesktopMetadata {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub categories: Option<Vec<String>>,
    pub icon: Option<String>,
}

#[derive(Debug)]
pub struct Extraction {
    pub metadata: DesktopMetadata,
    pub icon_path: Option<PathBuf>,
    pub root: PathBuf,
}

impl Drop for Extraction {
    fn drop(&mut self) {
        if self.root.exists() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

pub fn parse_desktop_entry(contents: &str) -> Option<DesktopMetadata> {
    let mut metadata = DesktopMetadata::default();
    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "Name" if !value.is_empty() => metadata.name = Some(value.to_string()),
            "Comment" if !value.is_empty() => metadata.comment = Some(value.to_string()),
            "Icon" if !value.is_empty() => metadata.icon = Some(value.to_string()),
            "Categories" => {
                let categories: Vec<_> = value
                    .split(';')
                    .filter(|category| !category.is_empty())
                    .map(str::to_string)
                    .collect();
                if !categories.is_empty() {
                    metadata.categories = Some(categories);
                }
            }
            _ => {}
        }
    }
    (metadata.name.is_some()
        || metadata.comment.is_some()
        || metadata.categories.is_some()
        || metadata.icon.is_some())
    .then_some(metadata)
}

pub fn is_appimage(path: &Path) -> Result<bool> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("appimage"));
    if extension {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() == 0 {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
        return Ok(true);
    }
    let bytes = fs::read(path)?;
    Ok(bytes.get(0..2) == Some(b"AI"))
}

pub fn inspect_and_extract(target: &Path) -> Result<Option<Extraction>> {
    if !is_appimage(target)? {
        return Ok(None);
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("musu-appimage-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&root)?;
    let mut attempts = 0;
    let output = loop {
        match Command::new(target)
            .arg("--appimage-extract")
            .current_dir(&root)
            .output()
        {
            Ok(output) if output.status.success() => break Some(output),
            Ok(_) => break None,
            Err(error) if attempts < 5 && error.raw_os_error() == Some(26) => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&root);
                return Err(error)
                    .with_context(|| format!("failed to inspect AppImage {}", target.display()));
            }
        }
    };
    let output = match output {
        Some(output) => output,
        None => {
            let _ = fs::remove_dir_all(&root);
            return Ok(None);
        }
    };
    let extracted_root = root.join("squashfs-root");
    if !extracted_root.is_dir() {
        let _ = fs::remove_dir_all(&root);
        return Ok(None);
    }
    let desktop = find_file(&extracted_root, |path| {
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("desktop"))
    });
    let metadata = desktop
        .as_deref()
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|contents| parse_desktop_entry(&contents))
        .unwrap_or_default();
    let icon_path = find_icon(&extracted_root, metadata.icon.as_deref());
    let _ = output;
    Ok(Some(Extraction {
        metadata,
        icon_path,
        root,
    }))
}

pub fn find_icon(root: &Path, requested: Option<&str>) -> Option<PathBuf> {
    if let Some(name) = requested {
        let direct = root.join(name);
        if direct.is_file() && crate::matcher::is_icon_file(&direct) {
            return Some(direct);
        }
        for ext in crate::matcher::ICON_EXTENSIONS {
            let candidate = root.join(format!("{name}.{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if let Some(found) = find_file(root, |path| {
            crate::matcher::is_icon_file(path)
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == name)
        }) {
            return Some(found);
        }
    }
    let dir_icon = root.join(".DirIcon");
    if dir_icon.is_file() {
        return Some(dir_icon);
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && crate::matcher::is_icon_file(&path) {
                return Some(path);
            }
        }
    }
    find_file(root, crate::matcher::is_icon_file)
}

fn find_file(root: &Path, predicate: impl Fn(&Path) -> bool + Copy) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && predicate(&path) {
            return Some(path);
        }
        if path.is_dir()
            && let Some(found) = find_file(&path, predicate)
        {
            return Some(found);
        }
    }
    None
}
