//! High-level application lifecycle orchestration.

use crate::{
    config, desktop, extractor, health,
    models::{AppEntry, Descriptions, slugify},
    symlink,
};
use anyhow::{Context, Result, bail};
use std::fmt::Write;
use std::fs;
use std::io::IsTerminal;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct AddOptions {
    pub target: PathBuf,
    pub name: Option<String>,
    pub icon: Option<PathBuf>,
    pub categories: Option<Vec<String>>,
    pub description: Option<String>,
    /// Per-language descriptions keyed by locale code (e.g. `es`, `fr`).
    pub descriptions: Option<Descriptions>,
    pub bin_name: Option<String>,
    pub no_desktop: bool,
    pub no_symlink: bool,
    pub dest_bin: Option<PathBuf>,
    /// Replace an existing registration with the same name/ID without
    /// prompting (equivalent to answering "overwrite" in the interactive
    /// collision menu).
    pub overwrite: bool,
    pub portable: bool,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateOptions {
    pub target: Option<PathBuf>,
    pub icon: Option<PathBuf>,
    pub name: Option<String>,
    pub categories: Option<Vec<String>>,
    pub description: Option<String>,
    /// Per-language descriptions keyed by locale code (e.g. `es`, `fr`).
    pub descriptions: Option<Descriptions>,
    pub remove_symlink: bool,
    pub portable: bool,
}

/// Split repeated `--description` values into the default description and
/// per-language descriptions (`LANG=text` entries, e.g. `es="Texto"`).
///
/// Returns `(default, languages)`. A value is treated as per-language only
/// when its `LANG` prefix is a valid locale code (2-10 chars, ASCII letter
/// first, then letters/digits/underscore); anything else is a plain
/// description (the last plain value wins).
pub fn parse_description_values(
    values: Vec<String>,
) -> Result<(Option<String>, Option<Descriptions>)> {
    let mut default = None;
    let mut languages = Descriptions::new();
    for value in values {
        match value.split_once('=') {
            Some((lang, text)) if crate::desktop::is_valid_language_code(lang) => {
                if !text.trim().is_empty() {
                    languages.insert(lang.to_string(), text.to_string());
                }
            }
            _ => {
                if !value.trim().is_empty() {
                    default = Some(value);
                }
            }
        }
    }
    Ok((default, (!languages.is_empty()).then_some(languages)))
}

/// Options for [`Manager::remove_with_options`].
#[derive(Debug, Clone, Default)]
pub struct RemoveOptions {
    /// Also delete the original target executable from disk (opt-in, destructive).
    pub delete_target: bool,
}

/// Sort order for [`Manager::list_with_options`].
#[derive(clap::ValueEnum, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortField {
    /// Sort by display name, A to Z (default).
    #[default]
    Name,
    /// Sort by display name, Z to A.
    NameDesc,
    /// Sort by id, A to Z.
    Id,
    /// Sort by id, Z to A.
    IdDesc,
    /// Sort by creation time, newest first.
    Created,
    /// Sort by first category, A to Z.
    Category,
}

#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub sort: SortField,
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

/// How to resolve a name/ID collision during [`Manager::add`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionAction {
    /// Register the new application under a different name.
    Rename,
    /// Remove the existing registration and replace it.
    Overwrite,
    /// Abort without touching anything (the safe default).
    Cancel,
}

/// An existing registration (or orphaned launcher) that collides
/// with a new `musu add`.
#[derive(Debug, Clone)]
pub struct AddCollision {
    /// Existing registry entry, when the ID is already managed.
    pub existing: Option<AppEntry>,
    /// Registry entries pointing at the same target binary under a
    /// *different* ID (duplicates created by moving or re-registering
    /// an app without removing the old entry).
    pub duplicates: Vec<AppEntry>,
    /// Existing `.desktop` file on disk (possibly orphaned from
    /// the registry).
    pub desktop_file: Option<PathBuf>,
}

impl AddCollision {
    /// Identifier of the colliding application, for messages.
    pub fn label(&self) -> String {
        self.existing
            .as_ref()
            .map(|app| app.id.clone())
            .or_else(|| self.duplicates.first().map(|app| app.id.clone()))
            .or_else(|| {
                self.desktop_file
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

/// Detect whether registering `entry` would collide with an already
/// registered application (same ID), with an orphaned `.desktop`
/// file left on disk, or with another registration pointing at the
/// same target binary.
pub fn detect_add_collision(
    registry: &crate::models::AppRegistry,
    data_dir: &Path,
    entry: &AppEntry,
) -> Option<AddCollision> {
    let existing = registry.find(&entry.id).cloned();
    let duplicates: Vec<AppEntry> = registry
        .apps
        .iter()
        .filter(|app| app.id != entry.id && same_target(&app.target_bin, &entry.target_bin))
        .cloned()
        .collect();
    let desktop_file = data_dir
        .join("applications")
        .join(format!("{}.desktop", entry.id));
    let desktop_file = desktop_file.is_file().then_some(desktop_file);
    (existing.is_some() || !duplicates.is_empty() || desktop_file.is_some()).then_some(
        AddCollision {
            existing,
            duplicates,
            desktop_file,
        },
    )
}

/// Whether two paths point at the same file, comparing canonicalized
/// paths when available and falling back to plain path equality for
/// missing targets.
fn same_target(a: &Path, b: &Path) -> bool {
    let canonical = |path: &Path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(a) == canonical(b)
}

/// Parse a collision-menu selection. Empty or unrecognized input
/// selects the safe default: [`CollisionAction::Cancel`].
pub fn parse_collision_choice(input: &str) -> CollisionAction {
    match input.trim().to_ascii_lowercase().as_str() {
        "r" | "rename" => CollisionAction::Rename,
        "o" | "overwrite" | "replace" => CollisionAction::Overwrite,
        _ => CollisionAction::Cancel,
    }
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
        let bundle_meta = crate::bundle::find_bundle_file(&canonical_target)
            .and_then(|file| crate::bundle::load_bundle(&file).ok());
        let bundle_dir = canonical_target.parent().unwrap_or(&canonical_target);
        let bundle_app = bundle_meta.as_ref().map(|b| &b.app);

        let resolved_bundle_icon = bundle_app.and_then(|app| {
            app.icon.as_ref().map(|icon_str| {
                crate::bundle::resolve_bundle_path(icon_str, bundle_dir, self.home_dir.as_deref())
            })
        });

        let extraction = if settings.defaults.auto_extract
            && (options.icon.is_none()
                || options.categories.is_none()
                || options.description.is_none()
                || options.descriptions.is_none())
        {
            extractor::inspect_and_extract(&canonical_target)?
        } else {
            None
        };
        let extracted_name = extraction
            .as_ref()
            .and_then(|result| result.metadata.name.as_deref());
        let bundle_name = bundle_app.map(|app| app.name.as_str());
        let mut entry = AppEntry::new(
            options.name.as_deref().or(bundle_name).or(extracted_name),
            canonical_target,
        );
        entry.description = options.description.or_else(|| {
            bundle_app
                .and_then(|app| app.description.clone())
                .or_else(|| {
                    extraction
                        .as_ref()
                        .and_then(|result| result.metadata.comment.clone())
                })
        });
        entry.descriptions = options.descriptions.or_else(|| {
            bundle_app
                .and_then(|app| app.descriptions.clone())
                .or_else(|| {
                    extraction
                        .as_ref()
                        .map(|result| &result.metadata.comment_langs)
                        .filter(|langs| !langs.is_empty())
                        .cloned()
                })
        });
        let detected_categories = if options.categories.is_none()
            && bundle_app.and_then(|app| app.categories.as_ref()).is_none()
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
            .or_else(|| bundle_app.and_then(|app| app.categories.clone()))
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
        // Resolve name/ID collisions before touching the filesystem:
        // rename, overwrite, or cancel.
        let mut registry = config::load(&self.config_path)?;
        while let Some(collision) = detect_add_collision(&registry, &self.data_dir, &entry) {
            if options.overwrite {
                self.remove_collision(&collision)?;
                registry = config::load(&self.config_path)?;
                continue;
            }
            if !std::io::stdin().is_terminal() {
                bail!(
                    "application '{}' already exists; pass --overwrite to replace it or --name to register it under a different name",
                    collision.label()
                );
            }
            match prompt_collision_action(&collision, &entry, self.home_dir.as_deref())? {
                CollisionAction::Rename => {
                    let Some(new_name) = prompt_new_name(&entry.name)? else {
                        bail!(
                            "aborted: '{}' already exists and was left unchanged",
                            collision.label()
                        );
                    };
                    entry.name = new_name;
                    entry.id = slugify(&entry.name);
                }
                CollisionAction::Overwrite => {
                    self.remove_collision(&collision)?;
                    registry = config::load(&self.config_path)?;
                }
                CollisionAction::Cancel => {
                    bail!(
                        "aborted: '{}' already exists and was left unchanged",
                        collision.label()
                    );
                }
            }
        }
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
        let target_exec = entry.target_bin.display().to_string();
        if !options.no_desktop {
            let (desktop_file, icon_path) = desktop::install(
                &self.data_dir,
                &entry.id,
                &entry.name,
                &target_exec,
                options
                    .icon
                    .as_deref()
                    .or(resolved_bundle_icon.as_deref())
                    .or_else(|| {
                        extraction
                            .as_ref()
                            .and_then(|result| result.icon_path.as_deref())
                    })
                    .or(probed_icon.as_deref()),
                entry.categories.as_deref(),
                desktop::Comments {
                    default: entry.description.as_deref(),
                    languages: entry.descriptions.as_ref(),
                },
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
            check_and_warn_path(destination);
        }
        if (options.portable || settings.defaults.portable)
            && let Some(parent) = entry.target_bin.parent()
        {
            let bin_rel = crate::bundle::serialize_bundle_path(
                &entry.target_bin,
                parent,
                self.home_dir.as_deref(),
            );
            let icon_str = options
                .icon
                .as_deref()
                .or(resolved_bundle_icon.as_deref())
                .map(|ic| {
                    crate::bundle::serialize_bundle_path(ic, parent, self.home_dir.as_deref())
                });
            let bundle_meta = crate::bundle::BundleMetadata {
                app: crate::bundle::BundleApp {
                    name: entry.name.clone(),
                    bin: bin_rel,
                    icon: icon_str,
                    description: entry.description.clone(),
                    descriptions: entry.descriptions.clone(),
                    categories: entry.categories.clone(),
                },
            };
            crate::bundle::save_bundle(parent, &bundle_meta)?;
        }
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
        let mut apps: Vec<_> = registry
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
        sort_apps(&mut apps, options.sort);
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
        if let Some(descriptions) = &app.descriptions {
            for (lang, text) in descriptions {
                writeln!(output, "    [{lang}]: {text}").unwrap();
            }
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
            || options.description.is_some()
            || options.descriptions.is_some()
            || options.portable;
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
            let mut extracted_comment_langs = None;
            if let Some(target) = options.target {
                let canonical = canonicalize_target(&target)?;
                entry.target_bin = canonical;
                if (options.icon.is_none()
                    || options.description.is_none()
                    || options.descriptions.is_none())
                    && extractor::is_appimage(&entry.target_bin).unwrap_or(false)
                    && let Ok(Some(extraction)) = extractor::inspect_and_extract(&entry.target_bin)
                {
                    extracted_icon = extraction.icon_path.clone();
                    extracted_comment = extraction.metadata.comment.clone();
                    extracted_comment_langs = Some(extraction.metadata.comment_langs.clone());
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
            if let Some(description) = options.description {
                entry.description = Some(description);
            } else if extracted_comment.is_some() {
                entry.description = extracted_comment;
            }
            // Merge the provided per-language descriptions into the existing
            // set: each `LANG=text` replaces just that language, leaving the
            // others untouched.
            if let Some(descriptions) = options.descriptions {
                let mut merged = entry.descriptions.clone().unwrap_or_default();
                for (lang, text) in descriptions {
                    if text.trim().is_empty() {
                        merged.remove(&lang);
                    } else {
                        merged.insert(lang, text);
                    }
                }
                entry.descriptions = (!merged.is_empty()).then_some(merged);
            } else if let Some(langs) = extracted_comment_langs.filter(|l| !l.is_empty()) {
                let mut merged = entry.descriptions.clone().unwrap_or_default();
                for (lang, text) in langs {
                    merged.entry(lang).or_insert(text);
                }
                entry.descriptions = Some(merged);
            }
            if options.icon.is_some() || extracted_icon.is_some() {
                desktop::remove(None, entry.icon_path.as_deref())?;
                entry.icon_path = None;
            }
            let target_exec = entry.target_bin.display().to_string();
            if let Some(desktop_file) = old.desktop_file.as_deref() {
                desktop::remove(Some(desktop_file), None)?;
                let (new_file, icon) = desktop::install(
                    &self.data_dir,
                    &entry.id,
                    &entry.name,
                    &target_exec,
                    options
                        .icon
                        .as_deref()
                        .or(extracted_icon.as_deref())
                        .or(entry.icon_path.as_deref()),
                    entry.categories.as_deref(),
                    desktop::Comments {
                        default: entry.description.as_deref(),
                        languages: entry.descriptions.as_ref(),
                    },
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
        if (options.portable || settings.defaults.portable)
            && let Some(parent) = updated.target_bin.parent()
        {
            let bin_rel = crate::bundle::serialize_bundle_path(
                &updated.target_bin,
                parent,
                self.home_dir.as_deref(),
            );
            let icon_str = options
                .icon
                .as_deref()
                .or(updated.icon_path.as_deref())
                .map(|ic| {
                    crate::bundle::serialize_bundle_path(ic, parent, self.home_dir.as_deref())
                });
            let bundle_meta = crate::bundle::BundleMetadata {
                app: crate::bundle::BundleApp {
                    name: updated.name.clone(),
                    bin: bin_rel,
                    icon: icon_str,
                    description: updated.description.clone(),
                    descriptions: updated.descriptions.clone(),
                    categories: updated.categories.clone(),
                },
            };
            crate::bundle::save_bundle(parent, &bundle_meta)?;
        }
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

    /// Remove the artifacts of a colliding registration (registry
    /// entry, desktop launcher, symlink, icon) while preserving
    /// the original target executable. Duplicate registrations that
    /// point at the same target binary are removed as well, so
    /// overwriting cannot leave a stale second launcher behind.
    fn remove_collision(&self, collision: &AddCollision) -> Result<()> {
        if let Some(existing) = &collision.existing {
            self.remove_with_options(&existing.id, RemoveOptions::default())?;
        }
        for duplicate in &collision.duplicates {
            self.remove_with_options(&duplicate.id, RemoveOptions::default())?;
        }
        if let Some(desktop_file) = &collision.desktop_file {
            desktop::remove(Some(desktop_file), None)?;
        }
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

/// Print the collision warning and read the user's decision from
/// the terminal. EOF cancels.
fn prompt_collision_action(
    collision: &AddCollision,
    new_entry: &AppEntry,
    home_dir: Option<&Path>,
) -> Result<CollisionAction> {
    use crate::colors::{BOLD_YELLOW, DIM_GRAY, RESET, TEAL};
    use std::io::Write;
    let label = collision.label();
    eprintln!(
        "{}Warning:{} an application with ID '{}' already exists.",
        BOLD_YELLOW, RESET, label
    );
    eprintln!();
    if !collision.duplicates.is_empty() {
        for duplicate in &collision.duplicates {
            eprintln!(
                "  Also registered as: {} ({}) -> {DIM_GRAY}{}{RESET}",
                duplicate.name,
                duplicate.id,
                collapse_path(&duplicate.target_bin, home_dir, false),
                DIM_GRAY = DIM_GRAY,
                RESET = RESET
            );
        }
        eprintln!();
    }
    if let Some(desktop_file) = &collision.desktop_file {
        eprintln!(
            "  Existing desktop: {DIM_GRAY}{}{RESET}",
            collapse_path(desktop_file, home_dir, false),
            DIM_GRAY = DIM_GRAY,
            RESET = RESET
        );
    }
    if let Some(existing) = &collision.existing {
        eprintln!(
            "  Existing target:  {DIM_GRAY}{}{RESET}",
            collapse_path(&existing.target_bin, home_dir, false),
            DIM_GRAY = DIM_GRAY,
            RESET = RESET
        );
    }
    eprintln!(
        "  {TEAL}New target:{RESET}       {DIM_GRAY}{}{RESET}",
        collapse_path(&new_entry.target_bin, home_dir, false),
        TEAL = TEAL,
        RESET = RESET,
        DIM_GRAY = DIM_GRAY
    );
    eprintln!();
    eprintln!("What would you like to do?");
    eprintln!("  [{}r{}] Rename new application", DIM_GRAY, RESET);
    if collision.duplicates.is_empty() {
        eprintln!(
            "  [{}o{}] Overwrite / replace existing application",
            DIM_GRAY, RESET
        );
    } else {
        eprintln!(
            "  [{}o{}] Overwrite / replace existing application (also removes the {} duplicate{})",
            DIM_GRAY,
            RESET,
            collision.duplicates.len(),
            if collision.duplicates.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }
    eprintln!("  [{}c{}] Cancel (default)", DIM_GRAY, RESET);
    eprintln!();
    eprint!(
        "Selection [{}r{}/{}o{}/{}C{}]: ",
        DIM_GRAY, RESET, DIM_GRAY, RESET, DIM_GRAY, RESET
    );
    std::io::stderr().flush()?;
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer)? == 0 {
        return Ok(CollisionAction::Cancel);
    }
    Ok(parse_collision_choice(&answer))
}

/// Prompt for a new display name after the user chose to rename.
/// Returns `None` on EOF (treated as cancel); empty input re-asks.
fn prompt_new_name(current: &str) -> Result<Option<String>> {
    use std::io::Write;
    loop {
        eprint!("Enter a new name for this application [{current}]: ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer)? == 0 {
            return Ok(None);
        }
        let trimmed = answer.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }
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
    if old.descriptions != new.descriptions {
        let format_langs = |langs: &Option<Descriptions>| {
            langs
                .as_ref()
                .map(|m| {
                    m.iter()
                        .map(|(lang, text)| format!("{lang}={text}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "none".to_string())
        };
        out.push_str(&format!(
            "  Descriptions: {} {} {}\n",
            format_langs(&old.descriptions),
            arrow,
            format_langs(&new.descriptions)
        ));
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

/// Sort the filtered application list in place according to `field`.
fn sort_apps(apps: &mut [&AppEntry], field: SortField) {
    match field {
        SortField::Name => apps.sort_by_key(|app| app.name.to_lowercase()),
        SortField::NameDesc => apps.sort_by_key(|app| std::cmp::Reverse(app.name.to_lowercase())),
        SortField::Id => apps.sort_by_key(|app| app.id.clone()),
        SortField::IdDesc => apps.sort_by_key(|app| std::cmp::Reverse(app.id.clone())),
        // Newest first.
        SortField::Created => apps.sort_by_key(|app| std::cmp::Reverse(app.created_at)),
        SortField::Category => apps.sort_by_key(|app| {
            app.categories
                .as_ref()
                .and_then(|c| c.first())
                .map(|s| s.to_lowercase())
                .unwrap_or_default()
        }),
    }
}
