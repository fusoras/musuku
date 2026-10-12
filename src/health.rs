//! Read-only diagnostics for desktop integration dependencies.

use crate::models::AppRegistry;
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct ToolCheck {
    pub name: String,
    pub available: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TerminalCheck {
    pub terminal: Option<String>,
    pub supported: bool,
    pub protocol: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PathCheck {
    pub name: String,
    pub path: PathBuf,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySummary {
    pub total: usize,
    pub healthy: usize,
    pub broken: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    pub terminal_graphics: TerminalCheck,
    pub svg_tools: Vec<ToolCheck>,
    pub desktop_tools: Vec<ToolCheck>,
    pub environment_paths: Vec<PathCheck>,
    pub app_summary: RegistrySummary,
}

impl HealthReport {
    pub fn collect(registry: &AppRegistry, data_dir: PathBuf) -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let local_bin = home.join(".local/bin");
        let applications = data_dir.join("applications");
        Self {
            terminal_graphics: detect_terminal_graphics(
                std::env::var("TERM_PROGRAM").ok().as_deref(),
                std::env::var("TERM").ok().as_deref(),
            ),
            // Only tools musu actually invokes: image.rs (SVG rasterizing)
            // and desktop.rs (launcher/icon cache syncing).
            svg_tools: check_tools(&["rsvg-convert", "magick", "convert"]),
            desktop_tools: check_tools(&["update-desktop-database", "gtk-update-icon-cache"]),
            environment_paths: vec![
                PathCheck {
                    name: "PATH".into(),
                    path: local_bin,
                    configured: std::env::split_paths(
                        &std::env::var_os("PATH").unwrap_or_default(),
                    )
                    .any(|path| path == home.join(".local/bin")),
                },
                PathCheck {
                    name: "XDG_DATA_DIRS".into(),
                    path: applications,
                    configured: data_dir_discoverable(
                        &data_dir,
                        std::env::var_os("XDG_DATA_HOME").as_deref(),
                        std::env::var_os("XDG_DATA_DIRS").as_deref(),
                        &crate::desktop::default_base_dir(),
                    ),
                },
            ],
            app_summary: RegistrySummary::from_registry(registry),
        }
    }

    /// Renders the report as human-readable text (capability-oriented).
    ///
    /// ANSI colors are decided at format time (see [`crate::colors::should_color`]);
    /// the [`HealthReport`] data itself stays color-free so `--json` stays clean.
    pub fn format_text(&self) -> String {
        self.format_text_with(crate::colors::should_color())
    }

    /// Renders the report as human-readable text, applying ANSI colors only
    /// when `color` is true. `format_text_with(false)` is always plain text.
    ///
    /// Each tool group is summarized as one capability line: the available
    /// tools in preference order, or a warning when the whole group is empty.
    /// A tool being absent while another of its group is present is normal
    /// operation, not a fault — use [`Self::format_text_verbose_with`] for the
    /// per-tool inventory.
    pub fn format_text_with(&self, color: bool) -> String {
        self.format_text_with_options(color, false)
    }

    /// Renders the per-tool inventory used by `musu health --verbose`.
    ///
    /// Every checked tool gets a line: `[ok] name (version)` when available,
    /// a dim `[–] name` when missing (never a warning for a single tool).
    /// Empty groups additionally show the capability warning of the default
    /// mode.
    pub fn format_text_verbose_with(&self, color: bool) -> String {
        self.format_text_with_options(color, true)
    }

    /// Shared renderer behind [`Self::format_text_with`] and
    /// [`Self::format_text_verbose_with`]; `verbose` only affects how the
    /// SVG and desktop tool groups are laid out.
    pub fn format_text_with_options(&self, color: bool, verbose: bool) -> String {
        let mut output = String::from("musu health\n\n");
        output.push_str("Terminal graphics:\n");
        if self.terminal_graphics.supported {
            output.push_str(&format!(
                "  {} your terminal supports the kitty graphics protocol\n",
                mark(true, color)
            ));
        } else {
            output.push_str(&format!(
                "  {} your terminal does not support the kitty graphics protocol\n",
                mark(false, color)
            ));
        }
        output.push_str("SVG converters:\n");
        write_tool_group(
            &mut output,
            &self.svg_tools,
            color,
            verbose,
            true,
            "no converter found — icon previews disabled (install rsvg-convert)",
        );
        output.push_str("Desktop utilities:\n");
        write_tool_group(
            &mut output,
            &self.desktop_tools,
            color,
            verbose,
            false,
            "no cache utility found — desktop/icon caches won't be refreshed (install update-desktop-database)",
        );
        output.push_str("Environment paths:\n");
        for path in &self.environment_paths {
            output.push_str(&format!(
                "  {} {} ({})\n",
                mark(path.configured, color),
                path.name,
                path.path.display()
            ));
        }
        // Only one count is emphasized: the broken tally in yellow when there
        // are broken entries, otherwise the healthy tally in green.
        let healthy = if self.app_summary.broken == 0 {
            crate::colors::green(color, &self.app_summary.healthy.to_string())
        } else {
            self.app_summary.healthy.to_string()
        };
        let broken = if self.app_summary.broken > 0 {
            crate::colors::yellow(color, &self.app_summary.broken.to_string())
        } else {
            self.app_summary.broken.to_string()
        };
        output.push_str(&format!(
            "Registry: {} healthy, {} broken, {} total\n",
            healthy, broken, self.app_summary.total
        ));
        output
    }
}

impl RegistrySummary {
    pub fn from_registry(registry: &AppRegistry) -> Self {
        let total = registry.apps.len();
        let healthy = registry
            .apps
            .iter()
            .filter(|app| app.target_bin.exists())
            .count();
        Self {
            total,
            healthy,
            broken: total - healthy,
        }
    }
}

/// Whether desktop environments can discover `data_dir` for launcher lookup.
///
/// The check keeps its historical `XDG_DATA_DIRS` name in the report, but the
/// criterion is the real XDG data search path: the data dir is discoverable if
/// it is the default data dir (implicitly covered by `$XDG_DATA_HOME`), if it
/// equals `$XDG_DATA_HOME`, or if it appears among the `$XDG_DATA_DIRS`
/// entries. Everything else is a non-standard, undiscoverable location.
///
/// Pure by design: environment values are passed in as parameters so tests
/// never mutate process-wide state (the crate forbids `unsafe_code`, which
/// rules out `env::set_var` on edition 2024).
pub fn data_dir_discoverable(
    data_dir: &Path,
    xdg_data_home: Option<&OsStr>,
    xdg_data_dirs: Option<&OsStr>,
    default_data_dir: &Path,
) -> bool {
    if data_dir == default_data_dir {
        return true;
    }
    if xdg_data_home.is_some_and(|home| Path::new(home) == data_dir) {
        return true;
    }
    xdg_data_dirs.is_some_and(|dirs| std::env::split_paths(dirs).any(|path| path == data_dir))
}

pub fn detect_terminal_graphics(term_program: Option<&str>, term: Option<&str>) -> TerminalCheck {
    let value = term_program
        .or(term)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let terminal = ["kitty", "ghostty", "wezterm", "herdr"]
        .iter()
        .find(|name| value.contains(**name))
        .map(|name| (*name).to_string());
    TerminalCheck {
        supported: terminal.is_some(),
        terminal,
        protocol: "Kitty Graphics Protocol",
    }
}

fn check_tools(names: &[&str]) -> Vec<ToolCheck> {
    names
        .iter()
        .map(|name| {
            let path = find_in_path(name);
            let version = path.as_ref().and_then(|path| {
                Command::new(path)
                    .arg("--version")
                    .output()
                    .ok()
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                    .filter(|version| !version.is_empty())
            });
            ToolCheck {
                name: (*name).to_string(),
                available: path.is_some(),
                path,
                version,
            }
        })
        .collect()
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn mark(ok: bool, color: bool) -> String {
    if ok {
        crate::colors::green(color, "[ok]")
    } else {
        crate::colors::yellow(color, "[warning]")
    }
}

/// Appends the body of a tool-group section (the header line is written by
/// the caller).
///
/// * `label_supported` — render the `[ok]` line as `supported (a, b)` (SVG
///   converters) instead of a bare comma-separated list (desktop utilities).
/// * `empty_warning` — consequence-oriented message shown when no tool of the
///   group is available; in verbose mode it is added on top of the `[–]` list.
///
/// Only tools musu actually invokes appear here, in their current preference
/// order (see `image.rs` and `desktop.rs`).
fn write_tool_group(
    output: &mut String,
    tools: &[ToolCheck],
    color: bool,
    verbose: bool,
    label_supported: bool,
    empty_warning: &str,
) {
    let available: Vec<&ToolCheck> = tools.iter().filter(|tool| tool.available).collect();
    if verbose {
        for tool in &available {
            let line = match tool.version.as_deref().and_then(short_version) {
                Some(version) => format!("{} ({version})", tool.name),
                None => tool.name.clone(),
            };
            output.push_str(&format!(
                "  {} {line}\n",
                crate::colors::green(color, "[ok]")
            ));
        }
        // A missing individual tool is normal operation (the group needs only
        // one converter/utility), so it is de-emphasized, never a warning.
        for tool in tools.iter().filter(|tool| !tool.available) {
            output.push_str(&format!(
                "  {} {}\n",
                crate::colors::dim(color, "[–]"),
                tool.name
            ));
        }
        if available.is_empty() {
            push_capability_warning(output, color, empty_warning);
        }
        return;
    }
    if available.is_empty() {
        push_capability_warning(output, color, empty_warning);
        return;
    }
    let names = available
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let marker = crate::colors::green(color, "[ok]");
    if label_supported {
        output.push_str(&format!("  {marker} supported ({names})\n"));
    } else {
        output.push_str(&format!("  {marker} {names}\n"));
    }
}

fn push_capability_warning(output: &mut String, color: bool, message: &str) {
    output.push_str(&format!(
        "  {} {message}\n",
        crate::colors::yellow(color, "[warning]")
    ));
}

/// Extracts a compact version for display from `--version` output: the first
/// whitespace-delimited token starting with a digit (so
/// `Version: ImageMagick 8.1.0-18 …` renders as `8.1.0-18`), falling back to
/// the first non-empty line when no such token exists.
fn short_version(raw: &str) -> Option<&str> {
    let first_line = raw.lines().map(str::trim).find(|line| !line.is_empty())?;
    Some(
        first_line
            .split_whitespace()
            .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(first_line),
    )
}
