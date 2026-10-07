//! High-level application lifecycle orchestration.

use crate::{
    config, desktop, extractor, health,
    models::{AppEntry, slugify},
    symlink,
};
use anyhow::{Context, Result, anyhow, bail};
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
    pub bin_name: Option<String>,
    pub no_desktop: bool,
    pub no_symlink: bool,
    pub dest_bin: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    pub target: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub name: Option<String>,
    pub categories: Option<Vec<String>>,
    pub remove_symlink: bool,
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
pub enum UpdateOutcome {
    NoChanges { name: String },
    Updated { entry: AppEntry },
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
            && (options.icon.is_none() || options.categories.is_none())
        {
            extractor::inspect_and_extract(&canonical_target)?
        } else {
            None
        };
        let extracted_name = extraction
            .as_ref()
            .and_then(|result| result.metadata.name.as_deref());
        let mut entry = AppEntry::new(options.name.as_deref().or(extracted_name), canonical_target);
        entry.categories = options
            .categories
            .or_else(|| {
                extraction
                    .as_ref()
                    .and_then(|result| result.metadata.categories.clone())
            })
            .or_else(|| Some(settings.defaults.categories.clone()))
            .map(|cats| {
                cats.into_iter()
                    .map(|c| crate::categories::normalize_category(&c))
                    .filter(|c| !c.is_empty())
                    .collect()
            });
        if !options.no_desktop {
            let (desktop_file, icon_path) = desktop::install(
                &self.data_dir,
                &entry.id,
                &entry.name,
                &entry.target_bin,
                options.icon.as_deref().or_else(|| {
                    extraction
                        .as_ref()
                        .and_then(|result| result.icon_path.as_deref())
                }),
                entry.categories.as_deref(),
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
            )?);
        }
        let mut registry = config::load(&self.config_path)?;
        registry.add(entry.clone());
        config::save(&self.config_path, &registry)?;
        if let Some(extraction) = extraction {
            let _ = fs::remove_dir_all(extraction.root);
        }
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
        if table {
            return Ok(format_table(
                &apps,
                show_icons,
                settings.display.list_icon_size,
                colored,
            ));
        }
        for app in apps {
            output.push_str("  ");
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
            .ok_or_else(|| anyhow!("application not found: {query}"))?;

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
            .ok_or_else(|| anyhow!("application not found: {query}"))
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
            .ok_or_else(|| anyhow!("application not found: {query}"))?;
        let metadata_change = options.target.is_some()
            || options.name.is_some()
            || options.icon.is_some()
            || options.categories.is_some();
        if !metadata_change && !options.remove_symlink {
            return Ok(UpdateOutcome::NoChanges { name: old.name });
        }
        let mut entry = old.clone();
        let mut extraction_root = None;
        if options.remove_symlink {
            if let Some(link) = old.symlink.as_deref() {
                symlink::remove(link)?;
            }
            entry.symlink = None;
        }
        if metadata_change {
            let mut extracted_icon = None;
            if let Some(target) = options.target {
                let canonical = canonicalize_target(&target)?;
                entry.target_bin = canonical;
                if options.icon.is_none()
                    && extractor::is_appimage(&entry.target_bin).unwrap_or(false)
                    && let Ok(Some(extraction)) = extractor::inspect_and_extract(&entry.target_bin)
                {
                    extracted_icon = extraction.icon_path;
                    extraction_root = Some(extraction.root);
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
            if options.icon.is_some() || extracted_icon.is_some() {
                desktop::remove(None, entry.icon_path.as_deref())?;
                entry.icon_path = None;
            }
            if let Some(desktop_file) = old.desktop_file.as_deref() {
                desktop::remove(Some(desktop_file), None)?;
                let (new_file, icon) = desktop::install(
                    &self.data_dir,
                    &entry.id,
                    &entry.name,
                    &entry.target_bin,
                    options
                        .icon
                        .as_deref()
                        .or(extracted_icon.as_deref())
                        .or(entry.icon_path.as_deref()),
                    entry.categories.as_deref(),
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
                )?);
            }
        } else if old.symlink.is_none() {
            // `--remove-symlink` on an app that never had one: nothing changed.
            return Ok(UpdateOutcome::NoChanges { name: old.name });
        }
        let updated = registry
            .update(query, |stored| *stored = entry.clone())
            .ok_or_else(|| anyhow!("application not found: {query}"))?
            .clone();
        config::save(&self.config_path, &registry)?;
        if let Some(root) = extraction_root {
            let _ = fs::remove_dir_all(root);
        }
        self.refresh_desktop_db(&settings);
        Ok(UpdateOutcome::Updated { entry: updated })
    }

    pub fn remove(&self, query: &str) -> Result<()> {
        self.remove_with_options(query, RemoveOptions::default())
    }

    pub fn remove_with_options(&self, query: &str, options: RemoveOptions) -> Result<()> {
        let mut registry = config::load(&self.config_path)?;
        let entry = registry
            .remove(query)
            .ok_or_else(|| anyhow!("application not found: {query}"))?;
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
            .ok_or_else(|| anyhow!("application not found: {query}"))?;
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
    icon_size: config::ListIconSize,
    colored: bool,
) -> String {
    let (cols, rows) = list_icon_dimensions(icon_size);
    let mut table = Vec::new();
    table.push(vec![
        "ICON".into(),
        "NAME".into(),
        "ID".into(),
        "CATEGORIES".into(),
        "STATUS".into(),
    ]);
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
        table.push(vec![
            icon,
            app.name.clone(),
            app.id.clone(),
            app.categories
                .as_ref()
                .map(|c| c.join(","))
                .unwrap_or_else(|| "-".into()),
            status_with_color(&app.target_bin, colored),
        ]);
    }
    let widths: Vec<usize> = (0..5)
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
