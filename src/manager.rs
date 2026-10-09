//! High-level application lifecycle orchestration.

use crate::{
    config, desktop, extractor, health,
    models::{AppEntry, slugify},
    sandbox, symlink,
};
use anyhow::{Context, Result, bail};
use std::fmt::Write;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    pub target: PathBuf,
    pub name: Option<String>,
    pub icon: Option<PathBuf>,
    pub categories: Option<Vec<String>>,
    pub description: Option<String>,
    pub bin_name: Option<String>,
    pub no_desktop: bool,
    pub no_symlink: bool,
    pub dest_bin: Option<PathBuf>,
    pub sandbox: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    pub target: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub name: Option<String>,
    pub categories: Option<Vec<String>>,
    pub description: Option<String>,
    pub remove_symlink: bool,
    pub sandbox: Option<bool>,
}

/// Options for [`Manager::remove_with_options`].
#[derive(Debug, Clone, Default)]
pub struct RemoveOptions {
    /// Also delete the original target executable from disk (opt-in, destructive).
    pub delete_target: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub icons: bool,
    pub no_icons: bool,
    pub image: bool,
    pub no_image: bool,
    pub table: bool,
    pub category: Option<String>,
    pub broken: bool,
    pub hide_disabled_symlinks: bool,
    pub hide_target_ok: bool,
    pub hide_desktop_ok: bool,
    pub hide_ok: bool,
    pub description: bool,
    pub no_description: bool,
    pub ids: bool,
    pub json: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ShowOptions {
    pub desktop: bool,
    pub bin: bool,
    pub icon: bool,
    pub symlink: bool,
    pub json: bool,
    pub absolute: bool,
    pub icons: bool,
    pub no_icons: bool,
    pub image: bool,
    pub no_image: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum UpdateOutcome {
    NoChanges { name: String },
    Updated { old: AppEntry, new: AppEntry },
}

#[derive(Debug, Clone)]
pub struct Manager {
    config_path: PathBuf,
    data_dir: PathBuf,
    bin_dir: PathBuf,
    home_dir: Option<PathBuf>,
}

impl Manager {
    pub fn new(config_path: PathBuf, data_dir: PathBuf, bin_dir: PathBuf) -> Self {
        Self {
            config_path,
            data_dir,
            bin_dir,
            home_dir: dirs::home_dir(),
        }
    }

    pub fn with_home(mut self, home: PathBuf) -> Self {
        self.home_dir = Some(home);
        self
    }

    pub fn add(&self, options: AddOptions) -> Result<AppEntry> {
        let settings = config::load_settings(self.settings_base())?;
        let canonical_target = canonicalize_target(&options.target)?;
        let extraction = if settings.defaults.auto_extract
            && (options.icon.is_none()
                || options.categories.is_none()
                || options.description.is_none())
        {
            extractor::inspect_and_extract(&canonical_target)?
        } else {
            None
        };
        let extracted_name = extraction
            .as_ref()
            .and_then(|result| result.metadata.name.as_deref());
        let mut entry = AppEntry::new(options.name.as_deref().or(extracted_name), canonical_target);
        entry.description = options.description.or_else(|| {
            extraction
                .as_ref()
                .and_then(|result| result.metadata.comment.clone())
        });
        let detected_categories = if options.categories.is_none()
            && extraction
                .as_ref()
                .and_then(|r| r.metadata.categories.as_ref())
                .is_none()
            && crate::matcher::is_renpy_game(&entry.target_bin)
        {
            Some(vec!["Game".to_string()])
        } else {
            None
        };
        entry.categories = options
            .categories
            .or_else(|| {
                extraction
                    .as_ref()
                    .and_then(|result| result.metadata.categories.clone())
            })
            .or(detected_categories)
            .or_else(|| Some(settings.defaults.categories.clone()))
            .map(|cats| {
                cats.into_iter()
                    .map(|c| crate::categories::normalize_category(&c))
                    .filter(|c| !c.is_empty())
                    .collect()
            });
        let probed_icon = if options.icon.is_none()
            && extraction
                .as_ref()
                .and_then(|r| r.icon_path.as_ref())
                .is_none()
        {
            crate::matcher::probe_app_icon(&entry.target_bin)
        } else {
            None
        };
        let sandbox_cmd =
            sandbox::generate_sandbox_exec(&settings, entry.sandbox, &entry.target_bin);
        if !options.no_desktop {
            let (desktop_file, icon_path) = desktop::install(
                &self.data_dir,
                &entry.id,
                &entry.name,
                &sandbox_cmd,
                options
                    .icon
                    .as_deref()
                    .or_else(|| {
                        extraction
                            .as_ref()
                            .and_then(|result| result.icon_path.as_deref())
                    })
                    .or(probed_icon.as_deref()),
                entry.categories.as_deref(),
                entry.description.as_deref(),
            )?;
            entry.desktop_file = Some(desktop_file);
            entry.icon_path = icon_path;
        }
        let should_symlink = if options.no_symlink {
            false
        } else if extractor::is_appimage(&entry.target_bin)? {
            settings.defaults.symlink_appimages
        } else {
            true
        };
        if should_symlink {
            let configured_dest =
                expand_home(&settings.defaults.dest_bin, self.home_dir.as_deref());
            let destination = options.dest_bin.as_deref().unwrap_or_else(|| {
                if settings.defaults.dest_bin == Path::new("~/.local/bin") {
                    &self.bin_dir
                } else {
                    &configured_dest
                }
            });
            entry.symlink = Some(symlink::install(
                destination,
                &entry.target_bin,
                options.bin_name.as_deref().unwrap_or(&entry.id),
                if entry.sandbox.unwrap_or(settings.sandbox.enabled) {
                    Some(sandbox_cmd.as_str())
                } else {
                    None
                },
            )?);
            check_and_warn_path(destination);
        }
        let mut registry = config::load(&self.config_path)?;
        registry.add(entry.clone());
        config::save(&self.config_path, &registry)?;
        drop(extraction);
        self.refresh_desktop_db(&settings);
        Ok(entry)
    }

    pub fn list(&self) -> Result<String> {
        self.list_with_options(ListOptions::default())
    }

    pub fn list_with_options(&self, options: ListOptions) -> Result<String> {
        let settings = config::load_settings(self.settings_base())?;
        let _ = crate::watcher::auto_sync_if_configured(self);
        let registry = config::load(&self.config_path)?;
        let mut output = String::new();
        let apps: Vec<_> = registry
            .apps
            .iter()
            .filter(|app| {
                options.category.as_deref().is_none_or(|category| {
                    app.categories.as_ref().is_some_and(|categories| {
                        categories
                            .iter()
                            .any(|value| value.eq_ignore_ascii_case(category))
                    })
                })
            })
            .filter(|app| !options.broken || !app.target_bin.exists())
            .collect();
        if options.json {
            return serde_json::to_string_pretty(&apps)
                .context("failed to serialize applications to JSON");
        }
        if options.ids {
            let mut ids = String::new();
            for app in apps {
                ids.push_str(&app.id);
                ids.push('\n');
            }
            return Ok(ids);
        }
        let table = options.table || settings.display.list_format == config::ListFormat::Table;
        let show_icons = crate::image::image_requested(
            settings.display.icons,
            options.icons || options.image,
            options.no_icons || options.no_image,
        );
        let hide_disabled =
            options.hide_disabled_symlinks || settings.display.hide_disabled_symlinks;
        let hide_target_ok = options.hide_target_ok
            || options.hide_ok
            || settings.display.hide_target_ok
            || settings.display.hide_ok_status;
        let hide_desktop_ok = options.hide_desktop_ok
            || options.hide_ok
            || settings.display.hide_desktop_ok
            || settings.display.hide_ok_status;
        let colored = crate::colors::should_color_policy(settings.display.color);
        let show_description =
            (options.description || settings.display.description) && !options.no_description;
        if table {
            return Ok(format_table(
                &apps,
                show_icons,
                show_description,
                settings.display.list_icon_size,
                colored,
            ));
        }
        for app in apps {
            output.push(' ');
            if show_icons {
                let rendered = app.icon_path.as_deref().and_then(|icon| {
                    let (cols, rows) = list_icon_dimensions(settings.display.list_icon_size);
                    crate::image::render_icon_preview(icon, cols, rows, true)
                });
                if let Some(preview) = rendered {
                    output.push_str(&preview);
                    output.push(' ');
                } else {
                    output.push_str("   ");
                }
            }
            let mut parts = Vec::new();
            parts.push(format!(
                "{} {}",
                app.name,
                crate::colors::gray(colored, &format!("({})", app.id))
            ));

            if show_description && let Some(desc) = &app.description {
                parts.push(format!("- {}", desc));
            }

            let target_is_ok = app.target_bin.exists();
            if !(hide_target_ok && target_is_ok) {
                parts.push(format!(
                    "target={}",
                    status_with_color(&app.target_bin, colored)
                ));
            }

            let desktop_is_ok = app
                .desktop_file
                .as_deref()
                .map(|p| p.exists())
                .unwrap_or(false);
            if !(hide_desktop_ok && desktop_is_ok) {
                parts.push(format!(
                    "desktop={}",
                    optional_status_with_color(app.desktop_file.as_deref(), colored)
                ));
            }

            if !(hide_disabled && app.symlink.is_none()) {
                parts.push(format!(
                    "symlink={}",
                    optional_status_with_color(app.symlink.as_deref(), colored)
                ));
            }

            writeln!(output, "{}", parts.join(" ")).unwrap();
        }
        Ok(output)
    }

    pub fn show(&self, query: &str, options: ShowOptions) -> Result<String> {
        let settings = config::load_settings(self.settings_base())?;
        let _ = crate::watcher::auto_sync_if_configured(self);
        let registry = config::load(&self.config_path)?;
        let app = registry
            .find(query)
            .ok_or_else(|| not_found_error(query, &registry))?;

        if options.json {
            return serde_json::to_string_pretty(app)
                .context("failed to serialize application as JSON");
        }

        let selected_paths = [
            (options.bin, app.target_bin.as_path(), "target executable"),
            (
                options.desktop,
                app.desktop_file.as_deref().unwrap_or(Path::new("")),
                "desktop file",
            ),
            (
                options.icon,
                app.icon_path.as_deref().unwrap_or(Path::new("")),
                "icon",
            ),
            (
                options.symlink,
                app.symlink.as_deref().unwrap_or(Path::new("")),
                "symlink",
            ),
        ];
        if let Some((_, path, label)) = selected_paths.iter().find(|(selected, _, _)| *selected) {
            if path.as_os_str().is_empty() {
                bail!("application has no {label}");
            }
            return Ok(format_path(
                path,
                self.home_dir.as_deref(),
                options.absolute,
                settings.display.paths,
            ));
        }

        let categories = app
            .categories
            .as_ref()
            .filter(|categories| !categories.is_empty())
            .map(|categories| categories.join(", "))
            .unwrap_or_else(|| "none".to_string());
        let mut output = String::new();
        if let Some(preview) = image_preview(
            app.icon_path.as_deref(),
            crate::image::image_requested(
                settings.display.icons,
                options.icons || options.image,
                options.no_icons || options.no_image,
            ),
            settings.display.show_icon_size,
        ) {
            output.push_str(&preview);
        }
        writeln!(
            output,
            "{}{} ({}){}",
            crate::colors::BOLD_BLUE,
            app.name,
            app.id,
            crate::colors::RESET
        )
        .unwrap();
        writeln!(
            output,
            "  Executable:  {} [{}]",
            format_path(
                &app.target_bin,
                self.home_dir.as_deref(),
                options.absolute,
                settings.display.paths
            ),
            status_with_color(
                &app.target_bin,
                crate::colors::should_color_policy(settings.display.color)
            )
        )
        .unwrap();
        writeln!(
            output,
            "  Desktop:     {}",
            optional_path_status(
                app.desktop_file.as_deref(),
                self.home_dir.as_deref(),
                options.absolute || settings.display.paths == config::PathPolicy::Absolute
            )
        )
        .unwrap();
        writeln!(
            output,
            "  Icon:        {}",
            optional_path_status(
                app.icon_path.as_deref(),
                self.home_dir.as_deref(),
                options.absolute || settings.display.paths == config::PathPolicy::Absolute
            )
        )
        .unwrap();
        writeln!(
            output,
            "  Symlink:     {}",
            optional_path_status(
                app.symlink.as_deref(),
                self.home_dir.as_deref(),
                options.absolute || settings.display.paths == config::PathPolicy::Absolute
            )
        )
        .unwrap();
        writeln!(output, "  Categories:  {categories}").unwrap();
        if let Some(desc) = &app.description {
            writeln!(output, "  Description: {desc}").unwrap();
        }
        writeln!(
            output,
            "  Registered:  {}",
            app.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
        .unwrap();
        Ok(output)
    }

    /// Look up a registered application by id or display name.
    pub fn get(&self, query: &str) -> Result<AppEntry> {
        let registry = config::load(&self.config_path)?;
        registry
            .find(query)
            .cloned()
            .ok_or_else(|| not_found_error(query, &registry))
    }

    pub fn is_registered_target(&self, target: &Path) -> Result<bool> {
        let target = fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
        Ok(config::load(&self.config_path)?
            .apps
            .iter()
            .any(|app| app.target_bin == target))
    }

    pub fn registered_target_query(&self, target: &Path) -> Result<Option<String>> {
        let target = fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
        Ok(config::load(&self.config_path)?
            .apps
            .into_iter()
            .find(|app| app.target_bin == target)
            .map(|app| app.id))
    }

    pub fn update(&self, query: &str, options: UpdateOptions) -> Result<UpdateOutcome> {
        let settings = config::load_settings(self.settings_base())?;
        let mut registry = config::load(&self.config_path)?;
        let old = registry
            .find(query)
            .cloned()
            .ok_or_else(|| not_found_error(query, &registry))?;
        let metadata_change = options.target.is_some()
            || options.name.is_some()
            || options.icon.is_some()
            || options.categories.is_some()
            || options.description.is_some();
        if !metadata_change && !options.remove_symlink {
            return Ok(UpdateOutcome::NoChanges { name: old.name });
        }
        let mut entry = old.clone();
        let mut _extraction_guard = None;
        if options.remove_symlink {
            if let Some(link) = old.symlink.as_deref() {
                symlink::remove(link)?;
            }
            entry.symlink = None;
        }
        if metadata_change {
            let mut extracted_icon = None;
            let mut extracted_comment = None;
            if let Some(target) = options.target {
                let canonical = canonicalize_target(&target)?;
                entry.target_bin = canonical;
                if (options.icon.is_none() || options.description.is_none())
                    && extractor::is_appimage(&entry.target_bin).unwrap_or(false)
                    && let Ok(Some(extraction)) = extractor::inspect_and_extract(&entry.target_bin)
                {
                    extracted_icon = extraction.icon_path.clone();
                    extracted_comment = extraction.metadata.comment.clone();
                    _extraction_guard = Some(extraction);
                }
            }
            if let Some(name) = options.name {
                entry.name = name;
                entry.id = slugify(&entry.name);
            }
            if let Some(categories) = options.categories {
                entry.categories = Some(
                    categories
                        .into_iter()
                        .map(|c| crate::categories::normalize_category(&c))
                        .filter(|c| !c.is_empty())
                        .collect(),
                );
            }
            if options.sandbox.is_some() {
                entry.sandbox = options.sandbox;
            }
            if let Some(description) = options.description {
                entry.description = Some(description);
            } else if extracted_comment.is_some() {
                entry.description = extracted_comment;
            }
            if options.icon.is_some() || extracted_icon.is_some() {
                desktop::remove(None, entry.icon_path.as_deref())?;
                entry.icon_path = None;
            }
            let sandbox_cmd =
                sandbox::generate_sandbox_exec(&settings, entry.sandbox, &entry.target_bin);
            if let Some(desktop_file) = old.desktop_file.as_deref() {
                desktop::remove(Some(desktop_file), None)?;
                let (new_file, icon) = desktop::install(
                    &self.data_dir,
                    &entry.id,
                    &entry.name,
                    &sandbox_cmd,
                    options
                        .icon
                        .as_deref()
                        .or(extracted_icon.as_deref())
                        .or(entry.icon_path.as_deref()),
                    entry.categories.as_deref(),
                    entry.description.as_deref(),
                )?;
                entry.desktop_file = Some(new_file);
                entry.icon_path = icon;
            }
            // `--remove-symlink` wins: the link is never regenerated, even when
            // the target or name changed in the same invocation.
            if let Some(link) = old.symlink.as_deref()
                && !options.remove_symlink
            {
                symlink::remove(link)?;
                let link_name = match link.file_name().and_then(|name| name.to_str()) {
                    Some(prev_name) if prev_name == old.id => &entry.id,
                    Some(prev_name) => prev_name,
                    None => &entry.id,
                };
                let destination =
                    configured_destination(&settings, &self.bin_dir, self.home_dir.as_deref());
                entry.symlink = Some(symlink::install(
                    &destination,
                    &entry.target_bin,
                    link_name,
                    if entry.sandbox.unwrap_or(settings.sandbox.enabled) {
                        Some(sandbox_cmd.as_str())
                    } else {
                        None
                    },
                )?);
                check_and_warn_path(&destination);
            }
        } else if old.symlink.is_none() {
            // `--remove-symlink` on an app that never had one: nothing changed.
            return Ok(UpdateOutcome::NoChanges { name: old.name });
        }
        let updated = registry
            .update(query, |stored| *stored = entry.clone())
            .ok_or_else(|| anyhow::anyhow!("Application '{}' not found.", query))?
            .clone();
        config::save(&self.config_path, &registry)?;
        drop(_extraction_guard);
        self.refresh_desktop_db(&settings);
        Ok(UpdateOutcome::Updated { old, new: updated })
    }

    pub fn remove(&self, query: &str) -> Result<()> {
        self.remove_with_options(query, RemoveOptions::default())
    }

    pub fn remove_with_options(&self, query: &str, options: RemoveOptions) -> Result<()> {
        let mut registry = config::load(&self.config_path)?;
        let entry = registry
            .remove(query)
            .ok_or_else(|| not_found_error(query, &registry))?;
        desktop::remove(entry.desktop_file.as_deref(), entry.icon_path.as_deref())?;
        if let Some(link) = entry.symlink.as_deref() {
            symlink::remove(link)?;
        }
        if options.delete_target && entry.target_bin.exists() {
            fs::remove_file(&entry.target_bin).with_context(|| {
                format!(
                    "failed to delete target executable {}",
                    entry.target_bin.display()
                )
            })?;
        }
        config::save(&self.config_path, &registry)?;
        self.refresh_desktop_db(&config::load_settings(self.settings_base())?);
        Ok(())
    }

    pub fn health(&self, json: bool, verbose: bool) -> Result<String> {
        let registry = config::load(&self.config_path)?;
        let report = health::HealthReport::collect(&registry, self.data_dir.clone());
        if json {
            Ok(serde_json::to_string_pretty(&report)?)
        } else if verbose {
            Ok(report.format_text_verbose_with(crate::colors::should_color()))
        } else {
            Ok(report.format_text())
        }
    }

    pub fn run(&self, query: &str, args: &[String]) -> Result<()> {
        use std::process::Stdio;
        let _ = crate::watcher::auto_sync_if_configured(self);
        let registry = config::load(&self.config_path)?;
        let app = registry
            .find(query)
            .ok_or_else(|| not_found_error(query, &registry))?;
        if !app.target_bin.exists() {
            bail!(
                "application target is missing: {}",
                app.target_bin.display()
            );
        }
        std::process::Command::new(&app.target_bin)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to launch {}", app.name))?;
        println!("Launched {} ({}) in background.", app.name, app.id);
        Ok(())
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn settings_base(&self) -> &Path {
        self.config_path.parent().unwrap_or_else(|| Path::new("."))
    }

    pub fn load_settings(&self) -> Result<config::Settings> {
        config::load_settings(self.settings_base())
    }

    pub fn save_settings(&self, settings: &config::Settings) -> Result<()> {
        config::save_settings(self.settings_base(), settings)
    }

    fn refresh_desktop_db(&self, settings: &config::Settings) {
        if settings.defaults.update_desktop_db {
            let applications_dir = self.data_dir.join("applications");
            let target = if applications_dir.is_dir() {
                &applications_dir
            } else {
                &self.data_dir
            };
            let _ = std::process::Command::new("update-desktop-database")
                .arg(target)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

/// Inline terminal image preview emitted before the `show` text card.
fn image_preview(
    icon_path: Option<&Path>,
    show_image: bool,
    size: config::ShowIconSize,
) -> Option<String> {
    if !show_image {
        return None;
    }
    let icon = icon_path?;
    let (cols, rows) = match size {
        config::ShowIconSize::Standard => (10, 5),
        config::ShowIconSize::Compact => (6, 3),
    };
    crate::image::render_icon_preview(icon, cols, rows, true).map(|mut esc| {
        esc.push('\n');
        esc
    })
}

fn canonicalize_target(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("target does not exist: {}", path.display()))?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        bail!("target is not a file: {}", canonical.display());
    }
    if metadata.permissions().mode() & 0o111 == 0 {
        bail!("target is not executable: {}", canonical.display());
    }
    Ok(canonical)
}

fn not_found_error(query: &str, registry: &crate::models::AppRegistry) -> anyhow::Error {
    let mut candidates = Vec::new();
    for app in &registry.apps {
        candidates.push(app.id.as_str());
        candidates.push(app.name.as_str());
    }
    if let Some(suggestion) = crate::matcher::suggest_similar_app(query, &candidates) {
        anyhow::anyhow!(
            "Application '{}' not found. Did you mean '{}'?",
            query,
            suggestion
        )
    } else {
        anyhow::anyhow!("Application '{}' not found.", query)
    }
}

fn status_with_color(path: &Path, colored: bool) -> String {
    if path.exists() {
        crate::colors::green(colored, "ok")
    } else {
        if colored {
            format!(
                "{}missing/broken{}",
                crate::colors::BOLD_RED,
                crate::colors::RESET
            )
        } else {
            "missing/broken".to_string()
        }
    }
}

fn optional_status_with_color(path: Option<&Path>, colored: bool) -> String {
    path.map(|path| status_with_color(path, colored))
        .unwrap_or_else(|| crate::colors::yellow(colored, "disabled"))
}

fn collapse_path(path: &Path, home_dir: Option<&Path>, absolute: bool) -> String {
    if absolute {
        return path.display().to_string();
    }
    if let Some(home) = home_dir
        && let Ok(suffix) = path.strip_prefix(home)
    {
        if suffix.as_os_str().is_empty() {
            return "~".to_string();
        }
        return format!("~/{}", suffix.display());
    }
    path.display().to_string()
}

fn optional_path_status(path: Option<&Path>, home_dir: Option<&Path>, absolute: bool) -> String {
    match path {
        Some(path) => format!(
            "{} [{}]",
            collapse_path(path, home_dir, absolute),
            status_with_color(path, crate::colors::should_color())
        ),
        None => format!(
            "{}disabled{}",
            crate::colors::BOLD_YELLOW,
            crate::colors::RESET
        ),
    }
}

fn list_icon_dimensions(size: config::ListIconSize) -> (u32, u32) {
    match size {
        config::ListIconSize::Compact => (2, 1),
        config::ListIconSize::Medium => (4, 2),
        config::ListIconSize::Large => (8, 4),
    }
}

fn format_path(
    path: &Path,
    home: Option<&Path>,
    absolute: bool,
    policy: config::PathPolicy,
) -> String {
    collapse_path(
        path,
        home,
        absolute || policy == config::PathPolicy::Absolute,
    )
}

fn expand_home(path: &Path, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else {
        return path.to_path_buf();
    };
    if path == Path::new("~") {
        return home.to_path_buf();
    }

    path.strip_prefix("~")
        .map(|suffix| home.join(suffix))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn configured_destination(
    settings: &config::Settings,
    fallback: &Path,
    home: Option<&Path>,
) -> PathBuf {
    if settings.defaults.dest_bin == Path::new("~/.local/bin") {
        fallback.to_path_buf()
    } else {
        expand_home(&settings.defaults.dest_bin, home)
    }
}

fn format_table(
    apps: &[&AppEntry],
    show_images: bool,
    show_description: bool,
    icon_size: config::ListIconSize,
    colored: bool,
) -> String {
    let (cols, rows) = list_icon_dimensions(icon_size);
    let mut table = Vec::new();
    let mut header = vec![
        "ICON".into(),
        "NAME".into(),
        "ID".into(),
        "CATEGORIES".into(),
    ];
    if show_description {
        header.push("DESCRIPTION".into());
    }
    header.push("STATUS".into());
    table.push(header);

    for app in apps {
        let icon = if show_images {
            app.icon_path
                .as_deref()
                .and_then(|path| crate::image::render_icon_preview(path, cols, rows, true))
                .map(|preview| format!("{preview}{}", " ".repeat(cols as usize)))
                .unwrap_or_else(|| " ".repeat(cols as usize))
        } else if app.icon_path.is_some() {
            "yes".into()
        } else {
            "-".into()
        };
        let mut row = vec![
            icon,
            app.name.clone(),
            app.id.clone(),
            app.categories
                .as_ref()
                .map(|c| c.join(","))
                .unwrap_or_else(|| "-".into()),
        ];
        if show_description {
            row.push(app.description.clone().unwrap_or_else(|| "-".into()));
        }
        row.push(status_with_color(&app.target_bin, colored));
        table.push(row);
    }
    let col_count = if show_description { 6 } else { 5 };
    let widths: Vec<usize> = (0..col_count)
        .map(|column| {
            table
                .iter()
                .map(|row| visible_width(&row[column]))
                .max()
                .unwrap_or(0)
        })
        .collect();
    let widths = widths
        .into_iter()
        .enumerate()
        .map(|(column, width)| {
            if column == 0 {
                width.max(cols as usize)
            } else {
                width
            }
        })
        .collect::<Vec<_>>();
    let border = crate::colors::dim(
        colored,
        &format!(
            "+{}+",
            widths
                .iter()
                .map(|w| format!("-{}-", w))
                .collect::<String>()
        ),
    );
    let mut output = String::new();
    for (index, row) in table.iter().enumerate() {
        let cells = row
            .iter()
            .enumerate()
            .map(|(column, cell)| centered_cell(cell, widths[column]))
            .collect::<Vec<_>>();
        writeln!(
            output,
            "{}{}{}",
            crate::colors::dim(colored, "|"),
            cells.join(&crate::colors::dim(colored, "|")),
            crate::colors::dim(colored, "|")
        )
        .unwrap();
        if index == 0 || index + 1 == table.len() {
            output.push_str(&border);
            output.push('\n');
        }
    }
    output
}

fn centered_cell(cell: &str, width: usize) -> String {
    let padding = width.saturating_sub(visible_width(cell));
    let left = padding / 2;
    format!(
        " {}{}{} ",
        " ".repeat(left),
        cell,
        " ".repeat(padding - left)
    )
}

fn visible_width(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut width = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b {
            width += 1;
            index += 1;
            continue;
        }

        index += 1;
        if index < bytes.len() && bytes[index] == b'[' {
            index += 1;
            while index < bytes.len() {
                let byte = bytes[index];
                index += 1;
                if byte.is_ascii_alphabetic() {
                    break;
                }
            }
        } else if index < bytes.len() && bytes[index] == b'_' {
            while index < bytes.len() {
                if bytes[index] == 0x1b && bytes.get(index + 1).copied() == Some(b'\\') {
                    index += 2;
                    break;
                }
                index += 1;
            }
        }
    }
    width
}

pub fn format_visual_diff(old: &AppEntry, new: &AppEntry) -> String {
    let mut out = String::new();
    let arrow = "➔";

    if old.target_bin != new.target_bin {
        out.push_str(&format!(
            "  Target: {} {} {}\n",
            old.target_bin.display(),
            arrow,
            new.target_bin.display()
        ));
    }
    if old.icon_path != new.icon_path {
        let old_icon = old
            .icon_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string());
        let new_icon = new
            .icon_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "none".to_string());
        out.push_str(&format!("  Icon: {} {} {}\n", old_icon, arrow, new_icon));
    }
    if old.name != new.name {
        out.push_str(&format!("  Name: {} {} {}\n", old.name, arrow, new.name));
    }
    if old.categories != new.categories {
        let old_cats = old
            .categories
            .as_ref()
            .map(|c| c.join(", "))
            .unwrap_or_else(|| "none".to_string());
        let new_cats = new
            .categories
            .as_ref()
            .map(|c| c.join(", "))
            .unwrap_or_else(|| "none".to_string());
        out.push_str(&format!(
            "  Categories: {} {} {}\n",
            old_cats, arrow, new_cats
        ));
    }
    if old.description != new.description {
        let old_desc = old.description.as_deref().unwrap_or("none");
        let new_desc = new.description.as_deref().unwrap_or("none");
        out.push_str(&format!(
            "  Description: {} {} {}\n",
            old_desc, arrow, new_desc
        ));
    }
    if old.sandbox != new.sandbox {
        let old_val = old
            .sandbox
            .map(|s| s.to_string())
            .unwrap_or_else(|| "none".to_string());
        let new_val = new
            .sandbox
            .map(|s| s.to_string())
            .unwrap_or_else(|| "none".to_string());
        out.push_str(&format!("  Sandbox: {} {} {}\n", old_val, arrow, new_val));
    }
    out
}

fn check_and_warn_path(destination: &Path) {
    let Ok(path_var) = std::env::var("PATH") else {
        return;
    };
    let dest_canon =
        std::fs::canonicalize(destination).unwrap_or_else(|_| destination.to_path_buf());

    let is_in_path = path_var.split(':').any(|p| {
        let p_path = Path::new(p);
        let p_canon = std::fs::canonicalize(p_path).unwrap_or_else(|_| p_path.to_path_buf());
        p_canon == dest_canon
    });

    if !is_in_path {
        let display = destination.display().to_string();
        let display_tilde = if let Some(home) = dirs::home_dir() {
            if let Ok(suffix) = destination.strip_prefix(&home) {
                format!("~/{}", suffix.display())
            } else {
                display.clone()
            }
        } else {
            display.clone()
        };
        eprintln!(
            "{}Note:{} '{}' is not in your current $PATH.",
            crate::colors::BOLD_YELLOW,
            crate::colors::RESET,
            display_tilde
        );
        eprintln!("Add this to your shell profile (~/.bashrc or ~/.zshrc):");
        eprintln!(
            "  export PATH=\"{}:$PATH\"",
            display_tilde.replace("~", "$HOME")
        );
    }
}
