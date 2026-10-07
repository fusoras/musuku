//! Batch scanner and debounced filesystem watcher for AppImages.

use crate::{
    cli::Watch,
    config, extractor,
    manager::{AddOptions, Manager},
    matcher,
};
use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

pub fn run(args: Watch, manager: Manager) -> Result<()> {
    let settings = config::load_settings(&config::default_base_dir())?;
    let watch = settings.watch;
    let directories = if args.directories.is_empty() {
        watch.directories.clone()
    } else {
        args.directories
    }
    .into_iter()
    .map(expand_home)
    .collect::<Vec<_>>();
    if directories.is_empty() {
        anyhow::bail!("no watch directories configured or provided");
    }
    let icons_dir = args.icons_dir.or(watch.icons_dir.clone()).map(expand_home);
    let recursive = args.recursive || watch.recursive;
    scan_directories(
        &manager,
        &directories,
        icons_dir.as_deref(),
        recursive,
        watch.auto_chmod,
        args.dry_run,
    )?;
    if args.scan_only {
        return Ok(());
    }
    watch_directories(
        manager,
        directories,
        icons_dir,
        recursive,
        watch.auto_chmod,
        watch.clean_on_delete,
        watch.debounce_ms,
    )
}

pub fn scan_directories(
    manager: &Manager,
    directories: &[PathBuf],
    icons_dir: Option<&Path>,
    recursive: bool,
    auto_chmod: bool,
    dry_run: bool,
) -> Result<usize> {
    let mut found = 0;
    for directory in directories {
        let mut paths = Vec::new();
        collect_appimages(directory, recursive, &mut paths)?;
        for path in paths {
            if manager.is_registered_target(&path)? {
                continue;
            }
            if auto_chmod {
                let mut permissions = fs::metadata(&path)?.permissions();
                permissions.set_mode(permissions.mode() | 0o111);
                fs::set_permissions(&path, permissions)?;
            }
            if !is_executable(&path)? {
                continue;
            }
            found += 1;
            let icon = icons_dir
                .map(|dir| matcher::find_icon(dir, path.to_string_lossy().as_ref()))
                .transpose()?
                .flatten();
            if !dry_run {
                manager.add(AddOptions {
                    target: path.clone(),
                    name: Some(matcher::normalize_name(
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or_default(),
                    )),
                    icon,
                    ..AddOptions::default()
                })?;
            }
        }
    }
    Ok(found)
}

fn watch_directories(
    manager: Manager,
    directories: Vec<PathBuf>,
    icons_dir: Option<PathBuf>,
    recursive: bool,
    auto_chmod: bool,
    clean_on_delete: bool,
    debounce_ms: u64,
) -> Result<()> {
    let (sender, receiver) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(debounce_ms), sender)
        .context("failed to initialize filesystem watcher")?;
    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };
    for directory in &directories {
        debouncer
            .watcher()
            .watch(directory, mode)
            .with_context(|| format!("failed to watch {}", directory.display()))?;
    }
    for result in receiver {
        handle_events(
            result,
            &manager,
            icons_dir.as_deref(),
            auto_chmod,
            clean_on_delete,
        )?;
    }
    Ok(())
}

fn handle_events(
    result: DebounceEventResult,
    manager: &Manager,
    icons_dir: Option<&Path>,
    auto_chmod: bool,
    clean_on_delete: bool,
) -> Result<()> {
    let events = result.context("filesystem watcher error")?;
    for event in events {
        let path = event.path;
        if path.is_file() && is_appimage_path(&path) {
            let parent = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let _ = scan_directories(
                manager,
                std::slice::from_ref(&parent),
                icons_dir,
                false,
                auto_chmod,
                false,
            )?;
        } else if clean_on_delete
            && !path.exists()
            && let Some(query) = manager.registered_target_query(&path)?
        {
            manager.remove(&query)?;
        }
    }
    Ok(())
}

fn collect_appimages(directory: &Path, recursive: bool, output: &mut Vec<PathBuf>) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read watch directory {}", directory.display()))?
    {
        let path = entry?.path();
        if path.is_file() && is_appimage_path(&path) {
            output.push(path);
        } else if recursive && path.is_dir() {
            collect_appimages(&path, true, output)?;
        }
    }
    Ok(())
}

fn is_appimage_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"))
}

fn is_executable(path: &Path) -> Result<bool> {
    Ok(fs::metadata(path)?.permissions().mode() & 0o111 != 0 || extractor::is_appimage(path)?)
}

fn expand_home(path: PathBuf) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path;
    };
    if (value == "~" || value.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        return home.join(value.strip_prefix("~/").unwrap_or_default());
    }
    path
}
