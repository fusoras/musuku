//! High-level application lifecycle orchestration.

use crate::{
    config, desktop,
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
}

#[derive(Debug, Clone, Default)]
pub struct ShowOptions {
    pub desktop: bool,
    pub bin: bool,
    pub icon: bool,
    pub symlink: bool,
    pub json: bool,
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
}

impl Manager {
    pub fn new(config_path: PathBuf, data_dir: PathBuf, bin_dir: PathBuf) -> Self {
        Self {
            config_path,
            data_dir,
            bin_dir,
        }
    }

    pub fn add(&self, options: AddOptions) -> Result<AppEntry> {
        let canonical_target = canonicalize_target(&options.target)?;
        let mut entry = AppEntry::new(options.name.as_deref(), canonical_target);
        entry.categories = options.categories.map(|cats| {
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
                options.icon.as_deref(),
                entry.categories.as_deref(),
            )?;
            entry.desktop_file = Some(desktop_file);
            entry.icon_path = icon_path;
        }
        if !options.no_symlink {
            entry.symlink = Some(symlink::install(
                options.dest_bin.as_deref().unwrap_or(&self.bin_dir),
                &entry.target_bin,
                options.bin_name.as_deref().unwrap_or(&entry.id),
            )?);
        }
        let mut registry = config::load(&self.config_path)?;
        registry.add(entry.clone());
        config::save(&self.config_path, &registry)?;
        Ok(entry)
    }

    pub fn list(&self) -> Result<String> {
        let registry = config::load(&self.config_path)?;
        let mut output = String::new();
        for app in registry.apps {
            writeln!(
                output,
                "{} {}({}){} target={} desktop={} symlink={}{}",
                app.name,
                crate::colors::DIM_GRAY,
                app.id,
                crate::colors::RESET,
                status(&app.target_bin),
                optional_status(app.desktop_file.as_deref()),
                optional_status(app.symlink.as_deref()),
                crate::colors::RESET,
            )
            .unwrap();
        }
        Ok(output)
    }

    pub fn show(&self, query: &str, options: ShowOptions) -> Result<String> {
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
            return Ok(path.display().to_string());
        }

        let categories = app
            .categories
            .as_ref()
            .filter(|categories| !categories.is_empty())
            .map(|categories| categories.join(", "))
            .unwrap_or_else(|| "none".to_string());
        let mut output = String::new();
        if let Some(preview) = image_preview(options.no_image) {
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
            app.target_bin.display(),
            status(&app.target_bin)
        )
        .unwrap();
        writeln!(
            output,
            "  Desktop:     {}",
            optional_path_status(app.desktop_file.as_deref())
        )
        .unwrap();
        writeln!(
            output,
            "  Icon:        {}",
            optional_path_status(app.icon_path.as_deref())
        )
        .unwrap();
        writeln!(
            output,
            "  Symlink:     {}",
            optional_path_status(app.symlink.as_deref())
        )
        .unwrap();
        writeln!(output, "  Categories:  {categories}").unwrap();
        writeln!(output, "  Registered:  {}", app.created_at).unwrap();
        Ok(output)
    }

    pub fn update(&self, query: &str, options: UpdateOptions) -> Result<UpdateOutcome> {
        let mut registry = config::load(&self.config_path)?;
        let old = registry
            .find(query)
            .cloned()
            .ok_or_else(|| anyhow!("application not found: {query}"))?;
        if options.target.is_none()
            && options.name.is_none()
            && options.icon.is_none()
            && options.categories.is_none()
        {
            return Ok(UpdateOutcome::NoChanges { name: old.name });
        }
        let mut entry = old.clone();
        if let Some(target) = options.target {
            let canonical = canonicalize_target(&target)?;
            entry.target_bin = canonical;
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
        if options.icon.is_some() {
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
                options.icon.as_deref().or(entry.icon_path.as_deref()),
                entry.categories.as_deref(),
            )?;
            entry.desktop_file = Some(new_file);
            entry.icon_path = icon;
        }
        if let Some(link) = old.symlink.as_deref() {
            symlink::remove(link)?;
            let link_name = match link.file_name().and_then(|name| name.to_str()) {
                Some(prev_name) if prev_name == old.id => &entry.id,
                Some(prev_name) => prev_name,
                None => &entry.id,
            };
            entry.symlink = Some(symlink::install(
                &self.bin_dir,
                &entry.target_bin,
                link_name,
            )?);
        }
        let updated = registry
            .update(query, |stored| *stored = entry.clone())
            .ok_or_else(|| anyhow!("application not found: {query}"))?
            .clone();
        config::save(&self.config_path, &registry)?;
        Ok(UpdateOutcome::Updated { entry: updated })
    }

    pub fn remove(&self, query: &str) -> Result<()> {
        let mut registry = config::load(&self.config_path)?;
        let entry = registry
            .remove(query)
            .ok_or_else(|| anyhow!("application not found: {query}"))?;
        desktop::remove(entry.desktop_file.as_deref(), entry.icon_path.as_deref())?;
        if let Some(link) = entry.symlink.as_deref() {
            symlink::remove(link)?;
        }
        config::save(&self.config_path, &registry)
    }
}

/// Inline terminal image preview emitted before the `show` text card.
///
/// `--no-image` suppresses the preview outright. Without it the preview would
/// be produced by the Kitty graphics renderer of the image engine, which this
/// crate does not wire in yet, so the text card alone is returned.
fn image_preview(no_image: bool) -> Option<String> {
    if no_image {
        return None;
    }
    None
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

fn status(path: &Path) -> String {
    if path.exists() {
        format!("{}ok{}", crate::colors::BOLD_GREEN, crate::colors::RESET)
    } else {
        format!(
            "{}missing/broken{}",
            crate::colors::BOLD_RED,
            crate::colors::RESET
        )
    }
}

fn optional_status(path: Option<&Path>) -> String {
    path.map(status).unwrap_or_else(|| {
        format!(
            "{}disabled{}",
            crate::colors::BOLD_YELLOW,
            crate::colors::RESET
        )
    })
}

fn optional_path_status(path: Option<&Path>) -> String {
    match path {
        Some(path) => format!("{} [{}]", path.display(), status(path)),
        None => format!(
            "{}disabled{}",
            crate::colors::BOLD_YELLOW,
            crate::colors::RESET
        ),
    }
}
