//! FreeDesktop launcher generation.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn default_base_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

pub fn install(
    base_dir: &Path,
    id: &str,
    name: &str,
    exec: &Path,
    icon: Option<&Path>,
    categories: Option<&[String]>,
) -> Result<(PathBuf, Option<PathBuf>)> {
    let applications = base_dir.join("applications");
    let icons = base_dir.join("icons");
    fs::create_dir_all(&applications)
        .with_context(|| format!("failed to create {}", applications.display()))?;
    let installed_icon = if let Some(source) = icon {
        fs::create_dir_all(&icons)
            .with_context(|| format!("failed to create {}", icons.display()))?;
        let file_name = source
            .file_name()
            .ok_or_else(|| anyhow!("icon path has no filename: {}", source.display()))?;
        let destination = icons.join(file_name);
        let same_file = source == destination
            || (destination.exists()
                && fs::canonicalize(source).ok() == fs::canonicalize(&destination).ok());
        if !same_file {
            fs::copy(source, &destination).with_context(|| {
                format!(
                    "failed to copy icon {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
        Some(destination)
    } else {
        None
    };

    let desktop_file = applications.join(format!("{id}.desktop"));
    let mut contents = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec={}\n",
        shell_escape(exec)
    );
    if let Some(icon) = installed_icon.as_deref() {
        contents.push_str(&format!("Icon={}\n", icon.display()));
    }
    if let Some(categories) = categories
        && !categories.is_empty()
    {
        contents.push_str(&format!("Categories={};\n", categories.join(";")));
    }
    contents.push_str("Terminal=false\n");
    fs::write(&desktop_file, contents)
        .with_context(|| format!("failed to write {}", desktop_file.display()))?;
    sync_desktop_database(&applications);
    sync_icon_cache(&icons);
    Ok((desktop_file, installed_icon))
}

pub fn remove(desktop_file: Option<&Path>, icon: Option<&Path>) -> Result<()> {
    for path in [desktop_file, icon].into_iter().flatten() {
        if path.exists() {
            fs::remove_file(path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
        }
    }
    if let Some(path) = desktop_file.and_then(Path::parent) {
        sync_desktop_database(path);
    }
    if let Some(path) = icon.and_then(Path::parent) {
        sync_icon_cache(path);
    }
    Ok(())
}

pub fn sync_desktop_database(applications_dir: &Path) {
    run_sync("update-desktop-database", applications_dir);
}

pub fn sync_icon_cache(icons_dir: &Path) {
    run_sync("gtk-update-icon-cache", icons_dir);
}

fn run_sync(program: &str, directory: &Path) {
    if !directory.is_dir() || find_in_path(program).is_none() {
        return;
    }
    let _ = Command::new(program)
        .arg(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn find_in_path(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

fn shell_escape(path: &Path) -> String {
    let value = path.to_string_lossy();
    if value.bytes().any(|byte| b" \t\n'\"".contains(&byte)) {
        format!("'{}'", value.replace('\'', "'\\''"))
    } else {
        value.into_owned()
    }
}
