//! Read-only diagnostics for desktop integration dependencies.

use crate::models::AppRegistry;
use serde::Serialize;
use std::path::PathBuf;
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
            svg_tools: check_tools(&["rsvg-convert", "magick", "convert", "inkscape"]),
            desktop_tools: check_tools(&[
                "update-desktop-database",
                "gtk-update-icon-cache",
                "xdg-icon-resource",
            ]),
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
                    configured: std::env::split_paths(
                        &std::env::var_os("XDG_DATA_DIRS").unwrap_or_default(),
                    )
                    .any(|path| path == data_dir),
                },
            ],
            app_summary: RegistrySummary::from_registry(registry),
        }
    }

    pub fn format_text(&self) -> String {
        let mut output = String::from("geico health\n\n");
        output.push_str(&format!(
            "Terminal graphics: {}\n",
            if self.terminal_graphics.supported {
                "[ok]"
            } else {
                "[warning]"
            }
        ));
        output.push_str("SVG converters:\n");
        for tool in &self.svg_tools {
            output.push_str(&format!("  {} {}\n", mark(tool.available), tool.name));
        }
        output.push_str("Desktop utilities:\n");
        for tool in &self.desktop_tools {
            output.push_str(&format!("  {} {}\n", mark(tool.available), tool.name));
        }
        output.push_str("Environment paths:\n");
        for path in &self.environment_paths {
            output.push_str(&format!(
                "  {} {} ({})\n",
                mark(path.configured),
                path.name,
                path.path.display()
            ));
        }
        output.push_str(&format!(
            "Registry: {} healthy, {} broken, {} total\n",
            self.app_summary.healthy, self.app_summary.broken, self.app_summary.total
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

fn mark(ok: bool) -> &'static str {
    if ok { "[ok]" } else { "[warning]" }
}
