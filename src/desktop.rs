//! FreeDesktop launcher generation.

use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::models::Descriptions;

/// Comments for a `.desktop` entry: the default `Comment=` line plus any
/// localized `Comment[lang]=` lines.
#[derive(Debug, Default, Clone, Copy)]
pub struct Comments<'a> {
    pub default: Option<&'a str>,
    pub languages: Option<&'a Descriptions>,
}

pub fn default_base_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/share")))
        .unwrap_or_else(|| PathBuf::from(".local/share"))
}

pub fn install(
    base_dir: &Path,
    id: &str,
    name: &str,
    exec: &str,
    icon: Option<&Path>,
    categories: Option<&[String]>,
    comments: Comments<'_>,
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

    let safe_id = id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect::<String>();
    let desktop_file = applications.join(format!("{safe_id}.desktop"));
    let safe_name = sanitize_entry_value(name);
    let safe_exec = sanitize_entry_value(exec);
    let mut contents = format!(
        "[Desktop Entry]\nType=Application\nName={safe_name}\nExec={}\n",
        escape_exec_value(&safe_exec)
    );
    if let Some(comment) = comments.default {
        let safe_comment = sanitize_entry_value(comment);
        if !safe_comment.is_empty() {
            contents.push_str(&format!("Comment={safe_comment}\n"));
        }
    }
    if let Some(comments) = comments.languages {
        for (lang, text) in comments {
            let safe_lang: String = lang
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let safe_text = sanitize_entry_value(text);
            if !safe_lang.is_empty() && !safe_text.is_empty() {
                contents.push_str(&format!("Comment[{safe_lang}]={safe_text}\n"));
            }
        }
    }
    if let Some(icon) = installed_icon.as_deref() {
        contents.push_str(&format!("Icon={}\n", icon.display()));
    }
    if let Some(categories) = categories {
        let safe_categories: Vec<String> = categories
            .iter()
            .map(|category| sanitize_entry_value(category).replace(';', ""))
            .filter(|category| !category.is_empty())
            .collect();
        if !safe_categories.is_empty() {
            contents.push_str(&format!("Categories={};\n", safe_categories.join(";")));
        }
    }
    contents.push_str("Terminal=false\n");
    fs::write(&desktop_file, contents)
        .with_context(|| format!("failed to write {}", desktop_file.display()))?;
    sync_desktop_database(&applications);
    sync_icon_cache(&icons);
    Ok((desktop_file, installed_icon))
}

pub fn sanitize_entry_value(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c == '\n' || c == '\r' || (c.is_control() && c != '\t') {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether the launcher at `desktop_file` embeds an `Exec=` line that
/// differs from the one [`install`] would write for `exec` today. The
/// comparison happens on the escaped/sanitized form, exactly as stored.
/// Missing or unreadable files are never reported as out of date — other
/// doctor checks cover those cases.
pub fn exec_out_of_date(desktop_file: &Path, exec: &str) -> bool {
    let Ok(contents) = fs::read_to_string(desktop_file) else {
        return false;
    };
    let Some(current) = contents.lines().find_map(|line| line.strip_prefix("Exec=")) else {
        return false;
    };
    current != escape_exec_value(&sanitize_entry_value(exec))
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

/// Whether `lang` is a plausible locale key for `Comment[lang]`
/// entries: starts with an ASCII letter, followed by letters,
/// digits, or underscores, 2-10 characters total (e.g. `es`,
/// `en`, `pt_BR`).
pub(crate) fn is_valid_language_code(lang: &str) -> bool {
    lang.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && (2..=10).contains(&lang.len())
        && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Format an `Exec=` value per the Desktop Entry specification: the command
/// and each of its arguments are separated by spaces; arguments containing
/// reserved characters (` `, tab, `"`, `'`, `\`, `` ` ``) are quoted with
/// **double quotes**, with `\`, `"`, `` ` `` and `$` backslash-escaped inside.
/// Quoting the whole line (e.g. with single quotes) is invalid — single
/// quotes are reserved characters — and makes launchers fail validation and
/// disappear from desktop menus.
fn escape_exec_value(exec: &str) -> String {
    exec.split_whitespace()
        .map(|arg| {
            let needs_quotes = arg.bytes().any(|byte| b" \t\"'\\`$".contains(&byte));
            if needs_quotes {
                let escaped = arg
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('`', "\\`")
                    .replace('$', "\\$");
                format!("\"{escaped}\"")
            } else {
                arg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
