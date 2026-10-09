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
    io::IsTerminal,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

pub fn sync_directories(
    manager: &Manager,
    directories: &[PathBuf],
    icons_dir: Option<&Path>,
    recursive: bool,
    auto_chmod: bool,
    clean_on_delete: bool,
    dry_run: bool,
) -> Result<SyncReport> {
    let mut resolved_dirs = Vec::new();
    for d in directories {
        if let Ok(resolved) = resolve_directory(d.clone()) {
            resolved_dirs.push(resolved);
        } else if d.is_dir() {
            resolved_dirs.push(d.clone());
        }
    }
    if resolved_dirs.is_empty() {
        return Ok(SyncReport::default());
    }

    let mut current_files = Vec::new();
    for dir in &resolved_dirs {
        collect_appimages(dir, recursive, &mut current_files)?;
    }

    struct Candidate {
        path: PathBuf,
        norm_name: String,
        version: Option<String>,
        mtime: Option<std::time::SystemTime>,
    }

    let mut candidates: Vec<Candidate> = Vec::new();
    for file in current_files {
        let canonical = fs::canonicalize(&file).unwrap_or(file);
        let filename = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let norm_name = matcher::normalize_name(filename);
        let version = matcher::extract_version(filename);
        let mtime = fs::metadata(&canonical)
            .ok()
            .and_then(|m| m.modified().ok());
        candidates.push(Candidate {
            path: canonical,
            norm_name,
            version,
            mtime,
        });
    }

    let mut claimed_paths: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    let mut report = SyncReport::default();

    let registry = config::load(manager.config_path())?;
    for app in &registry.apps {
        let in_watched_dir = resolved_dirs.iter().any(|d| app.target_bin.starts_with(d));
        if !in_watched_dir {
            continue;
        }

        let app_norm = matcher::normalize_name(&app.id);

        if !app.target_bin.exists() {
            // Target was deleted from disk! Check if there is an alternative version candidate.
            let matching_candidate = candidates
                .iter()
                .filter(|c| c.norm_name == app_norm)
                .max_by(|a, b| match (&a.version, &b.version) {
                    (Some(v1), Some(v2)) => matcher::compare_versions(v1, v2),
                    _ => a.mtime.cmp(&b.mtime),
                });

            if let Some(cand) = matching_candidate {
                if !dry_run {
                    let outcome = manager.update(
                        &app.id,
                        crate::manager::UpdateOptions {
                            target: Some(cand.path.clone()),
                            ..Default::default()
                        },
                    )?;
                    if let crate::manager::UpdateOutcome::Updated { old, new } = outcome {
                        println!(
                            "{}Auto-upgraded '{}'{}",
                            crate::colors::BOLD_BLUE,
                            new.name,
                            crate::colors::RESET
                        );
                        print!("{}", crate::manager::format_visual_diff(&old, &new));
                    }
                }
                claimed_paths.insert(cand.path.clone());
                report.updated += 1;
            } else if clean_on_delete {
                if !dry_run {
                    manager.remove(&app.id)?;
                }
                report.removed += 1;
            }
        } else {
            // Target exists; check if a newer version candidate is available
            let app_filename = app
                .target_bin
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let app_version = matcher::extract_version(app_filename);
            let app_mtime = fs::metadata(&app.target_bin)
                .ok()
                .and_then(|m| m.modified().ok());

            let newer_candidate = candidates
                .iter()
                .filter(|c| c.norm_name == app_norm && c.path != app.target_bin)
                .find(|c| match (&c.version, &app_version) {
                    (Some(v_new), Some(v_old)) => {
                        matcher::compare_versions(v_new, v_old) == std::cmp::Ordering::Greater
                    }
                    _ => c.mtime > app_mtime,
                });

            if let Some(cand) = newer_candidate {
                if !dry_run {
                    let outcome = manager.update(
                        &app.id,
                        crate::manager::UpdateOptions {
                            target: Some(cand.path.clone()),
                            ..Default::default()
                        },
                    )?;
                    if let crate::manager::UpdateOutcome::Updated { old, new } = outcome {
                        println!(
                            "{}Auto-upgraded '{}'{}",
                            crate::colors::BOLD_BLUE,
                            new.name,
                            crate::colors::RESET
                        );
                        print!("{}", crate::manager::format_visual_diff(&old, &new));
                    }
                }
                claimed_paths.insert(cand.path.clone());
                claimed_paths.insert(app.target_bin.clone());
                report.updated += 1;
            } else {
                claimed_paths.insert(app.target_bin.clone());
            }
        }
    }

    // Register new candidates
    for cand in candidates {
        if claimed_paths.contains(&cand.path) {
            continue;
        }
        if manager.is_registered_target(&cand.path)? {
            continue;
        }
        if auto_chmod && let Ok(metadata) = fs::metadata(&cand.path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0o111);
            let _ = fs::set_permissions(&cand.path, permissions);
        }
        if !is_executable(&cand.path)? {
            continue;
        }
        let icon = icons_dir
            .map(|dir| matcher::find_icon(dir, cand.path.to_string_lossy().as_ref()))
            .transpose()?
            .flatten();
        if !dry_run {
            manager.add(AddOptions {
                target: cand.path.clone(),
                name: Some(cand.norm_name.clone()),
                icon,
                ..AddOptions::default()
            })?;
        }
        report.added += 1;
    }

    Ok(report)
}

fn sync_cache_path(base_dir: &Path) -> PathBuf {
    base_dir.join(".sync_cache")
}

fn read_last_sync(base_dir: &Path) -> Option<std::time::SystemTime> {
    let content = fs::read_to_string(sync_cache_path(base_dir)).ok()?;
    let millis: u64 = content.trim().parse().ok()?;
    Some(std::time::UNIX_EPOCH + Duration::from_millis(millis))
}

fn write_last_sync(base_dir: &Path, time: std::time::SystemTime) {
    if let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) {
        let _ = fs::write(sync_cache_path(base_dir), duration.as_millis().to_string());
    }
}

fn get_directory_mtime(dir: &Path, recursive: bool) -> Option<std::time::SystemTime> {
    let mut max_mtime = fs::metadata(dir).ok()?.modified().ok()?;
    if recursive && let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(sub_mtime) = get_directory_mtime(&path, true)
            {
                max_mtime = max_mtime.max(sub_mtime);
            }
        }
    }
    Some(max_mtime)
}

fn is_sync_needed(
    manager: &Manager,
    directories: &[PathBuf],
    recursive: bool,
    base_dir: &Path,
) -> bool {
    let Some(last_sync) = read_last_sync(base_dir) else {
        return true;
    };

    if let Ok(config_mtime) =
        fs::metadata(config::settings_path(base_dir)).and_then(|m| m.modified())
        && config_mtime > last_sync
    {
        return true;
    }

    for dir in directories {
        let expanded = expand_home(dir.clone());
        let Ok(canonical) = fs::canonicalize(&expanded) else {
            return true;
        };
        let Some(dir_mtime) = get_directory_mtime(&canonical, recursive) else {
            return true;
        };
        if dir_mtime > last_sync {
            return true;
        }
    }

    let Ok(registry) = config::load(manager.config_path()) else {
        return true;
    };
    for app in &registry.apps {
        for dir in directories {
            let expanded = expand_home(dir.clone());
            if let Ok(canonical) = fs::canonicalize(&expanded)
                && app.target_bin.starts_with(&canonical)
                && !app.target_bin.exists()
            {
                return true;
            }
        }
    }

    false
}

pub fn auto_sync_if_configured(manager: &Manager) -> Result<Option<SyncReport>> {
    let base_dir = manager.settings_base();
    let settings = config::load_settings(base_dir)?;
    if !settings.watch.auto_sync || settings.watch.directories.is_empty() {
        return Ok(None);
    }
    if !is_sync_needed(
        manager,
        &settings.watch.directories,
        settings.watch.recursive,
        base_dir,
    ) {
        return Ok(None);
    }
    let _spinner = crate::spinner::Spinner::start("Syncing watch directories...");
    let icons_dir = settings
        .watch
        .icons_dir
        .as_ref()
        .map(|d| expand_home(d.clone()))
        .and_then(|d| {
            if d.is_dir() {
                fs::canonicalize(&d).ok()
            } else {
                None
            }
        });
    let report = sync_directories(
        manager,
        &settings.watch.directories,
        icons_dir.as_deref(),
        settings.watch.recursive,
        settings.watch.auto_chmod,
        settings.watch.clean_on_delete,
        false,
    )?;
    write_last_sync(base_dir, std::time::SystemTime::now());
    Ok(Some(report))
}

pub fn run(args: Watch, manager: Manager) -> Result<()> {
    crate::service::ensure_systemd_unit_exists();

    let base_dir = manager.settings_base().to_path_buf();
    let mut settings = config::load_settings(&base_dir)?;

    if let Some(unwatch_dirs) = &args.unwatch {
        let mut modified = false;
        for dir in unwatch_dirs {
            let resolved = resolve_directory(dir.clone()).unwrap_or_else(|_| dir.clone());
            let prev_len = settings.watch.directories.len();
            settings
                .watch
                .directories
                .retain(|d| d != dir && d != &resolved);
            if settings.watch.directories.len() < prev_len {
                modified = true;
                println!("Removed '{}' from watch directories.", dir.display());
            } else {
                println!("Directory '{}' was not in watch list.", dir.display());
            }
        }
        if modified {
            config::save_settings(&base_dir, &settings)?;
        }
        if args.directories.is_empty() && !args.continuous {
            return Ok(());
        }
    }

    let raw_directories = if args.directories.is_empty() {
        settings.watch.directories.clone()
    } else {
        args.directories.clone()
    };

    if raw_directories.is_empty() {
        anyhow::bail!("no watch directories configured or provided");
    }

    let mut directories = Vec::new();
    for dir in &raw_directories {
        let resolved = resolve_directory(dir.clone())?;
        directories.push(resolved);
    }

    // Persist new explicitly passed directories to config.toml so list will auto-sync them
    if !args.directories.is_empty() {
        let mut modified = false;
        for dir in &directories {
            if !settings.watch.directories.contains(dir) {
                settings.watch.directories.push(dir.clone());
                modified = true;
            }
        }
        if modified {
            let _ = config::save_settings(&base_dir, &settings);
        }
    }

    let icons_dir = args
        .icons_dir
        .or(settings.watch.icons_dir.clone())
        .map(expand_home)
        .and_then(|dir| {
            if dir.is_dir() {
                fs::canonicalize(&dir).ok()
            } else {
                None
            }
        });

    let recursive = args.recursive || settings.watch.recursive;

    let spinner = crate::spinner::Spinner::start_with_delay(
        "Scanning and syncing watch directories...",
        std::time::Duration::ZERO,
        std::io::stderr().is_terminal(),
    );
    let report = sync_directories(
        &manager,
        &directories,
        icons_dir.as_deref(),
        recursive,
        settings.watch.auto_chmod,
        settings.watch.clean_on_delete,
        args.dry_run,
    )?;
    if std::io::stderr().is_terminal() && !spinner.has_rendered() {
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    spinner.stop();
    write_last_sync(&base_dir, std::time::SystemTime::now());

    println!(
        "Sync complete: {} registered, {} updated, {} removed.",
        report.added, report.updated, report.removed
    );

    if args.continuous {
        if settings.watch.auto_chmod {
            eprintln!(
                "{}Warning: auto_chmod is enabled. Executable permissions will be automatically granted to detected AppImages.{}",
                crate::colors::BOLD_YELLOW,
                crate::colors::RESET
            );
        }
        println!(
            "\n{}Watching for AppImages (press Ctrl+C to stop):{}",
            crate::colors::BOLD_BLUE,
            crate::colors::RESET
        );
        for dir in &directories {
            println!("  • {}", dir.display());
        }

        watch_directories(
            manager,
            directories,
            icons_dir,
            recursive,
            settings.watch.auto_chmod,
            settings.watch.clean_on_delete,
            settings.watch.debounce_ms,
        )?;
    }

    Ok(())
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
            if auto_chmod && let Ok(metadata) = fs::metadata(&path) {
                let mut permissions = metadata.permissions();
                if permissions.mode() & 0o111 == 0 {
                    permissions.set_mode(permissions.mode() | 0o111);
                    if let Err(err) = fs::set_permissions(&path, permissions) {
                        eprintln!(
                            "musu: warning: failed to set chmod +x on {}: {err}",
                            path.display()
                        );
                    } else {
                        println!(
                            "{}Granted executable permissions to {}{}",
                            crate::colors::BOLD_YELLOW,
                            path.display(),
                            crate::colors::RESET
                        );
                    }
                }
            }
            if !is_executable(&path).unwrap_or(false) {
                continue;
            }
            found += 1;
            let icon = icons_dir
                .map(|dir| matcher::find_icon(dir, path.to_string_lossy().as_ref()))
                .transpose()?
                .flatten();
            let app_name = matcher::normalize_name(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default(),
            );
            if !dry_run {
                let entry = match manager.add(AddOptions {
                    target: path.clone(),
                    name: Some(app_name.clone()),
                    icon,
                    ..AddOptions::default()
                }) {
                    Ok(entry) => entry,
                    Err(err) => {
                        eprintln!(
                            "{}musu: warning: skipping {}: {err:#}{}",
                            crate::colors::BOLD_YELLOW,
                            path.display(),
                            crate::colors::RESET
                        );
                        continue;
                    }
                };
                println!(
                    "{}Registered {} ({}) from {}{}",
                    crate::colors::BOLD_GREEN,
                    entry.name,
                    entry.id,
                    path.display(),
                    crate::colors::RESET
                );
            } else {
                println!(
                    "[dry-run] Would register {} from {}",
                    app_name,
                    path.display()
                );
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
        if let Err(err) = handle_events(
            result,
            &manager,
            icons_dir.as_deref(),
            auto_chmod,
            clean_on_delete,
        ) {
            eprintln!("musu: warning: watcher loop error: {err:#}");
        }
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
    let events = match result {
        Ok(events) => events,
        Err(err) => {
            eprintln!("musu: warning: filesystem watcher event error: {err:#}");
            return Ok(());
        }
    };
    for event in events {
        let path = event.path;
        if is_temp_or_download_path(&path) {
            continue;
        }
        if path.is_file() && is_appimage_path(&path) {
            let parent = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            if let Err(err) = scan_directories(
                manager,
                std::slice::from_ref(&parent),
                icons_dir,
                false,
                auto_chmod,
                false,
            ) {
                eprintln!(
                    "musu: warning: error scanning {}: {err:#}",
                    parent.display()
                );
            }
        } else if clean_on_delete
            && !path.exists()
            && let Ok(Some(query)) = manager.registered_target_query(&path)
            && let Ok(entry) = manager.get(&query)
        {
            if let Err(err) = manager.remove(&query) {
                eprintln!("musu: warning: failed to remove {}: {err:#}", entry.name);
            } else {
                println!(
                    "{}Removed {} ({}) - AppImage deleted from disk{}",
                    crate::colors::BOLD_YELLOW,
                    entry.name,
                    entry.id,
                    crate::colors::RESET
                );
            }
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
            let is_hidden_dir = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            if !is_hidden_dir {
                collect_appimages(&path, true, output)?;
            }
        }
    }
    Ok(())
}

pub fn is_temp_or_download_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if name.starts_with('.') || name.ends_with('~') {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".part")
        || lower.ends_with(".crdownload")
        || lower.ends_with(".tmp")
        || lower.ends_with(".temp")
        || lower.ends_with(".download")
        || lower.ends_with(".aria2")
        || lower.ends_with(".swp")
}

pub fn is_appimage_path(path: &Path) -> bool {
    if is_temp_or_download_path(path) {
        return false;
    }
    let has_appimage_ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("appimage"));
    if !has_appimage_ext {
        return false;
    }
    fs::metadata(path).is_ok_and(|meta| meta.len() > 0)
}

fn is_executable(path: &Path) -> Result<bool> {
    if is_temp_or_download_path(path) {
        return Ok(false);
    }
    let Ok(meta) = fs::metadata(path) else {
        return Ok(false);
    };
    if meta.len() == 0 {
        return Ok(false);
    }
    Ok(meta.permissions().mode() & 0o111 != 0 || extractor::is_appimage(path).unwrap_or(false))
}

fn resolve_directory(path: PathBuf) -> Result<PathBuf> {
    let expanded = expand_home(path);
    if !expanded.exists() {
        anyhow::bail!("directory does not exist: {}", expanded.display());
    }
    if !expanded.is_dir() {
        anyhow::bail!("path is not a directory: {}", expanded.display());
    }
    Ok(fs::canonicalize(&expanded)?)
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
