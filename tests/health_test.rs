use geico::health::{
    HealthReport, RegistrySummary, ToolCheck, data_dir_discoverable, detect_terminal_graphics,
};
use geico::models::AppRegistry;
use std::ffi::OsStr;
use std::path::PathBuf;

fn tool(name: &str, available: bool) -> ToolCheck {
    ToolCheck {
        name: name.into(),
        available,
        path: None,
        version: None,
    }
}

/// Report with empty env paths / registry and a supported terminal, so any
/// `[warning]` in the output must come from a tool group.
fn capability_report(svg_tools: Vec<ToolCheck>, desktop_tools: Vec<ToolCheck>) -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("ghostty"), None),
        svg_tools,
        desktop_tools,
        sandbox_tools: vec![tool("bwrap", true)],
        environment_paths: vec![],
        app_summary: RegistrySummary {
            total: 0,
            healthy: 0,
            broken: 0,
        },
    }
}

fn supported_report() -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("ghostty"), None),
        svg_tools: vec![],
        desktop_tools: vec![],
        sandbox_tools: vec![tool("bwrap", true)],
        environment_paths: vec![],
        app_summary: RegistrySummary {
            total: 0,
            healthy: 0,
            broken: 0,
        },
    }
}

fn unsupported_report() -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("xterm-256color"), Some("xterm-256color")),
        svg_tools: vec![],
        desktop_tools: vec![],
        sandbox_tools: vec![tool("bwrap", true)],
        environment_paths: vec![],
        app_summary: RegistrySummary {
            total: 0,
            healthy: 0,
            broken: 0,
        },
    }
}

#[test]
fn test_health_text_graphics_supported_reports_ok_message() {
    let text = supported_report().format_text_with(false);
    assert!(
        text.contains("Terminal graphics:"),
        "expected 'Terminal graphics:' section header in:\n{text}"
    );
    assert!(
        text.contains("  [ok] your terminal supports the kitty graphics protocol"),
        "expected indented [ok] line in:\n{text}"
    );
    assert!(
        text.contains("kitty graphics protocol"),
        "expected kitty graphics protocol wording in:\n{text}"
    );
}

#[test]
fn test_health_text_graphics_section_layout_matches_contract() {
    let text = supported_report().format_text_with(false);
    let lines: Vec<&str> = text.lines().take(4).collect();
    assert_eq!(
        lines,
        vec![
            "geico health",
            "",
            "Terminal graphics:",
            "  [ok] your terminal supports the kitty graphics protocol",
        ],
        "unexpected section layout in:\n{text}"
    );
}

#[test]
fn test_health_text_output_contains_no_emoji_markers() {
    for report in [supported_report(), unsupported_report(), rich_report()] {
        let text = report.format_text_with(false);
        assert!(!text.contains('✅'), "no ✅ allowed in:\n{text}");
        assert!(!text.contains('⚠'), "no ⚠ allowed in:\n{text}");
    }
}

#[test]
fn test_health_text_graphics_supported_omits_detected_terminal_name() {
    let text = supported_report().format_text();
    assert!(
        !text.contains("ghostty"),
        "detected terminal name must not be printed in:\n{text}"
    );
}

#[test]
fn test_health_text_graphics_unsupported_reports_warning_message() {
    let text = unsupported_report().format_text_with(false);
    assert!(
        text.contains("Terminal graphics:"),
        "expected 'Terminal graphics:' section header in:\n{text}"
    );
    assert!(
        text.contains("  [warning] your terminal does not support the kitty graphics protocol"),
        "expected indented [warning] line in:\n{text}"
    );
    assert!(
        text.contains("kitty graphics protocol"),
        "expected kitty graphics protocol wording in:\n{text}"
    );
    assert!(
        !text.contains("xterm"),
        "detected terminal name must not be printed in:\n{text}"
    );
}

fn rich_report() -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("kitty"), None),
        svg_tools: vec![geico::health::ToolCheck {
            name: "rsvg-convert".into(),
            available: true,
            path: None,
            version: None,
        }],
        desktop_tools: vec![geico::health::ToolCheck {
            name: "xdg-icon-resource".into(),
            available: false,
            path: None,
            version: None,
        }],
        sandbox_tools: vec![geico::health::ToolCheck {
            name: "bwrap".into(),
            available: true,
            path: None,
            version: None,
        }],
        environment_paths: vec![geico::health::PathCheck {
            name: "PATH".into(),
            path: PathBuf::from("/home/user/.local/bin"),
            configured: false,
        }],
        app_summary: RegistrySummary {
            total: 2,
            healthy: 1,
            broken: 1,
        },
    }
}

fn clean_registry_report() -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("kitty"), None),
        svg_tools: vec![],
        desktop_tools: vec![],
        sandbox_tools: vec![tool("bwrap", true)],
        environment_paths: vec![],
        app_summary: RegistrySummary {
            total: 1,
            healthy: 1,
            broken: 0,
        },
    }
}

#[test]
fn test_health_text_plain_mode_keeps_bracket_markers() {
    // Contract change: `[warning]` is no longer emitted per missing tool —
    // it now comes from an empty capability group (here: the desktop group,
    // whose only tool is unavailable). `[ok]` marks a satisfied capability.
    let text = rich_report().format_text_with(false);
    assert!(text.contains("[ok]"), "expected [ok] marker in:\n{text}");
    assert!(
        text.contains("[warning]"),
        "expected [warning] marker in:\n{text}"
    );
}

#[test]
fn test_health_text_plain_mode_graphics_lines_are_exact_literals() {
    let supported = supported_report().format_text_with(false);
    assert!(
        supported
            .lines()
            .any(|line| line == "  [ok] your terminal supports the kitty graphics protocol"),
        "expected exact supported literal in:\n{supported}"
    );
    let unsupported = unsupported_report().format_text_with(false);
    assert!(
        unsupported
            .lines()
            .any(|line| line
                == "  [warning] your terminal does not support the kitty graphics protocol"),
        "expected exact unsupported literal in:\n{unsupported}"
    );
}

#[test]
fn test_health_text_plain_mode_has_no_escape_codes() {
    let text = rich_report().format_text_with(false);
    assert!(
        !text.contains('\x1b'),
        "plain mode must not emit ANSI codes:\n{text:?}"
    );
}

#[test]
fn test_health_text_colored_mode_wraps_ok_marker_in_green() {
    let text = rich_report().format_text_with(true);
    assert!(
        text.contains("\x1b[1;32m[ok]\x1b[0m"),
        "expected green [ok] marker in:\n{text:?}"
    );
}

#[test]
fn test_health_text_colored_mode_wraps_warning_marker_in_yellow() {
    let text = rich_report().format_text_with(true);
    assert!(
        text.contains("\x1b[1;33m[warning]\x1b[0m"),
        "expected yellow [warning] marker in:\n{text:?}"
    );
}

#[test]
fn test_health_text_colored_mode_highlights_graphics_status_word() {
    let supported = supported_report().format_text_with(true);
    assert!(
        supported
            .contains("\x1b[1;32m[ok]\x1b[0m your terminal supports the kitty graphics protocol"),
        "expected green [ok] graphics line in:\n{supported:?}"
    );
    let unsupported = unsupported_report().format_text_with(true);
    assert!(
        unsupported.contains(
            "\x1b[1;33m[warning]\x1b[0m your terminal does not support the kitty graphics protocol"
        ),
        "expected yellow [warning] graphics line in:\n{unsupported:?}"
    );
}

#[test]
fn test_health_text_colored_mode_highlights_broken_registry_count() {
    let text = rich_report().format_text_with(true);
    assert!(
        text.contains("Registry: 1 healthy, \x1b[1;33m1\x1b[0m broken, 2 total"),
        "expected yellow broken count in:\n{text:?}"
    );
}

#[test]
fn test_health_text_colored_mode_highlights_healthy_registry_when_clean() {
    let text = clean_registry_report().format_text_with(true);
    assert!(
        text.contains("Registry: \x1b[1;32m1\x1b[0m healthy, 0 broken, 1 total"),
        "expected green healthy count in:\n{text:?}"
    );
}

#[test]
fn test_health_json_output_contains_no_ansi_escapes() {
    let report = rich_report();
    let json = serde_json::to_string(&report).unwrap();
    assert!(
        !json.contains('\x1b'),
        "JSON output must never carry ANSI codes:\n{json}"
    );
}

#[test]
fn test_health_reports_kitty_terminal_support() {
    let report = detect_terminal_graphics(Some("kitty"), None);
    assert!(report.supported);
    assert_eq!(report.terminal.as_deref(), Some("kitty"));
}

#[test]
fn test_health_json_output_is_valid_schema() {
    let report = HealthReport::collect(&AppRegistry::new(), PathBuf::from("/tmp/geico-test"));
    let json = serde_json::to_value(report).unwrap();
    assert!(json.get("svg_tools").is_some());
    assert!(json.get("desktop_tools").is_some());
    assert!(json.get("environment_paths").is_some());
    assert_eq!(
        json["app_summary"],
        serde_json::json!({"total": 0, "healthy": 0, "broken": 0})
    );
}

#[test]
fn test_health_registry_summary_counts_broken_targets() {
    let mut registry = AppRegistry::new();
    registry.add(geico::models::AppEntry::new(
        Some("missing"),
        PathBuf::from("/definitely/missing"),
    ));
    let summary = RegistrySummary::from_registry(&registry);
    assert_eq!(summary.total, 1);
    assert_eq!(summary.healthy, 0);
    assert_eq!(summary.broken, 1);
}

#[test]
fn test_health_svg_tools_only_lists_converters_the_program_invokes() {
    let report = HealthReport::collect(&AppRegistry::new(), PathBuf::from("/tmp/geico-test"));
    let names: Vec<&str> = report
        .svg_tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["rsvg-convert", "magick", "convert"],
        "svg_tools must match the converters invoked by image.rs"
    );
    assert!(
        !names.contains(&"inkscape"),
        "inkscape is never invoked by geico and must not be listed: {names:?}"
    );
}

#[test]
fn test_health_desktop_tools_only_lists_utilities_the_program_invokes() {
    let report = HealthReport::collect(&AppRegistry::new(), PathBuf::from("/tmp/geico-test"));
    let names: Vec<&str> = report
        .desktop_tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    assert_eq!(
        names,
        ["update-desktop-database", "gtk-update-icon-cache"],
        "desktop_tools must match the utilities invoked by desktop.rs"
    );
    assert!(
        !names.contains(&"xdg-icon-resource"),
        "xdg-icon-resource is never invoked by geico and must not be listed: {names:?}"
    );
}

#[test]
fn test_data_dir_discoverable_default_data_dir_returns_true() {
    let default = PathBuf::from("/home/user/.local/share");
    assert!(
        data_dir_discoverable(&default, None, None, &default),
        "the default data dir must count as discoverable (implicit XDG_DATA_HOME)"
    );
}

#[test]
fn test_data_dir_discoverable_matches_xdg_data_home_returns_true() {
    let data_dir = PathBuf::from("/custom/data");
    let default = PathBuf::from("/home/user/.local/share");
    assert!(
        data_dir_discoverable(&data_dir, Some(OsStr::new("/custom/data")), None, &default),
        "data dir equal to $XDG_DATA_HOME must be discoverable"
    );
}

#[test]
fn test_data_dir_discoverable_listed_in_xdg_data_dirs_returns_true() {
    let data_dir = PathBuf::from("/opt/share");
    let default = PathBuf::from("/home/user/.local/share");
    assert!(
        data_dir_discoverable(
            &data_dir,
            None,
            Some(OsStr::new("/usr/local/share:/opt/share:/usr/share")),
            &default
        ),
        "data dir listed in $XDG_DATA_DIRS must be discoverable"
    );
}

#[test]
fn test_data_dir_discoverable_unknown_data_dir_returns_false() {
    let data_dir = PathBuf::from("/weird/place");
    let default = PathBuf::from("/home/user/.local/share");
    assert!(
        !data_dir_discoverable(
            &data_dir,
            None,
            Some(OsStr::new("/usr/local/share:/usr/share")),
            &default
        ),
        "non-standard data dir absent from all XDG variables must warn"
    );
}

#[test]
fn test_health_collect_default_data_dir_marks_xdg_check_configured() {
    let data_dir = geico::desktop::default_base_dir();
    let report = HealthReport::collect(&AppRegistry::new(), data_dir.clone());
    let check = report
        .environment_paths
        .iter()
        .find(|check| check.name == "XDG_DATA_DIRS")
        .expect("XDG_DATA_DIRS check must be present");
    assert_eq!(check.path, data_dir.join("applications"));
    assert!(
        check.configured,
        "default data dir must not produce a false-positive warning"
    );
}

// ---------------------------------------------------------------------------
// Capability-oriented report (default mode): a group is only warned about
// when NO tool of the group is available. Individual missing tools are never
// a warning in default mode (see `--verbose` for the per-tool inventory).
// ---------------------------------------------------------------------------

#[test]
fn test_health_default_group_with_available_tools_reports_supported_ok() {
    let report = capability_report(
        vec![
            tool("rsvg-convert", false),
            tool("magick", true),
            tool("convert", true),
        ],
        vec![
            tool("update-desktop-database", true),
            tool("gtk-update-icon-cache", true),
        ],
    );
    let text = report.format_text_with(false);
    assert!(
        text.contains("  [ok] supported (magick, convert)\n"),
        "expected capability line with available converters only in:\n{text}"
    );
    assert!(
        !text.contains("[warning]"),
        "no warning allowed when every group has an available tool in:\n{text}"
    );
}

#[test]
fn test_health_default_svg_group_without_converter_reports_install_warning() {
    let report = capability_report(
        vec![
            tool("rsvg-convert", false),
            tool("magick", false),
            tool("convert", false),
        ],
        vec![tool("update-desktop-database", true)],
    );
    let text = report.format_text_with(false);
    assert!(
        text.contains(
            "  [warning] no converter found — icon previews disabled (install rsvg-convert)\n"
        ),
        "expected empty-SVG-group warning in:\n{text}"
    );
}

#[test]
fn test_health_default_desktop_group_without_utility_reports_install_warning() {
    let report = capability_report(
        vec![tool("magick", true)],
        vec![
            tool("update-desktop-database", false),
            tool("gtk-update-icon-cache", false),
        ],
    );
    let text = report.format_text_with(false);
    assert!(
        text.contains(
            "  [warning] no cache utility found — desktop/icon caches won't be refreshed (install update-desktop-database)\n"
        ),
        "expected empty-desktop-group warning in:\n{text}"
    );
}

#[test]
fn test_health_default_desktop_group_lists_only_available_tools() {
    let report = capability_report(
        vec![tool("magick", true)],
        vec![
            tool("update-desktop-database", true),
            tool("gtk-update-icon-cache", false),
        ],
    );
    let text = report.format_text_with(false);
    assert!(
        text.contains("  [ok] update-desktop-database\n"),
        "expected available utility listed in:\n{text}"
    );
    assert!(
        !text.contains("gtk-update-icon-cache"),
        "missing tools must not be listed in default mode:\n{text}"
    );
    assert!(
        !text.contains("[warning]"),
        "an available tool must not trigger a warning in:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// --verbose / -v: per-tool inventory. Missing tools use the dim `[–]` marker,
// never `[warning]`; an empty group additionally shows the capability warning.
// ---------------------------------------------------------------------------

#[test]
fn test_health_verbose_lists_every_tool_and_dims_missing_ones() {
    let mut magick = tool("magick", true);
    magick.version = Some("8.1.0-18".into());
    let mut convert = tool("convert", true);
    convert.version = Some("6.9.12-58".into());
    let report = capability_report(
        vec![tool("rsvg-convert", false), magick, convert],
        vec![
            tool("update-desktop-database", true),
            tool("gtk-update-icon-cache", true),
        ],
    );
    let text = report.format_text_with_options(false, true);
    assert!(
        text.contains("  [ok] magick (8.1.0-18)\n"),
        "expected available tool with version in:\n{text}"
    );
    assert!(
        text.contains("  [ok] convert (6.9.12-58)\n"),
        "expected available tool with version in:\n{text}"
    );
    assert!(
        text.contains("  [ok] update-desktop-database\n"),
        "expected available tool without version in:\n{text}"
    );
    assert!(
        text.contains("  [–] rsvg-convert\n"),
        "expected dim dash marker for missing tool in:\n{text}"
    );
    assert!(
        !text.contains("[warning]"),
        "verbose must not warn about individually missing tools in:\n{text}"
    );
    let ok_index = text
        .find("  [ok] magick")
        .expect("available tool line must exist");
    let dash_index = text
        .find("  [–] rsvg-convert")
        .expect("missing tool line must exist");
    assert!(
        ok_index < dash_index,
        "available tools must be listed before missing ones in:\n{text}"
    );
}

#[test]
fn test_health_verbose_empty_groups_add_capability_warning() {
    let report = capability_report(
        vec![
            tool("rsvg-convert", false),
            tool("magick", false),
            tool("convert", false),
        ],
        vec![
            tool("update-desktop-database", false),
            tool("gtk-update-icon-cache", false),
        ],
    );
    let text = report.format_text_with_options(false, true);
    for name in [
        "rsvg-convert",
        "magick",
        "convert",
        "update-desktop-database",
        "gtk-update-icon-cache",
    ] {
        assert!(
            text.contains(&format!("  [–] {name}\n")),
            "expected dim dash line for {name} in:\n{text}"
        );
    }
    assert!(
        text.contains(
            "[warning] no converter found — icon previews disabled (install rsvg-convert)"
        ),
        "expected SVG capability warning in verbose mode:\n{text}"
    );
    assert!(
        text.contains(
            "[warning] no cache utility found — desktop/icon caches won't be refreshed (install update-desktop-database)"
        ),
        "expected desktop capability warning in verbose mode:\n{text}"
    );
}

#[test]
fn test_health_verbose_plain_mode_has_no_escape_codes() {
    let report = capability_report(
        vec![tool("rsvg-convert", false), tool("magick", true)],
        vec![tool("update-desktop-database", false)],
    );
    let text = report.format_text_with_options(false, true);
    assert!(
        !text.contains('\x1b'),
        "verbose plain mode must not emit ANSI codes:\n{text:?}"
    );
}

#[test]
fn test_health_verbose_colored_mode_wraps_markers_in_expected_codes() {
    let report = capability_report(
        vec![
            tool("rsvg-convert", false),
            tool("magick", true),
            tool("convert", false),
        ],
        vec![tool("update-desktop-database", false)],
    );
    let text = report.format_text_with_options(true, true);
    assert!(
        text.contains("\x1b[1;32m[ok]\x1b[0m magick"),
        "expected green [ok] for available tool in:\n{text:?}"
    );
    assert!(
        text.contains("\x1b[90m[–]\x1b[0m rsvg-convert"),
        "expected dim gray [–] for missing tool in:\n{text:?}"
    );
    let svg_section = text
        .split("SVG converters:\n")
        .nth(1)
        .and_then(|rest| rest.split("Desktop utilities:").next())
        .expect("SVG section must exist");
    assert!(
        !svg_section.contains("[warning]"),
        "non-empty SVG group must not warn in:\n{text:?}"
    );
}

// ---------------------------------------------------------------------------
// CLI flag plumbing.
// ---------------------------------------------------------------------------

#[test]
fn test_health_cli_accepts_verbose_flag_short_and_long() {
    use clap::Parser;
    use geico::cli::{Cli, Command};

    for flag in ["--verbose", "-v"] {
        let cli = Cli::try_parse_from(["geico", "health", flag])
            .unwrap_or_else(|error| panic!("health must accept {flag}: {error}"));
        match cli.command {
            Command::Health(args) => {
                assert!(args.verbose, "{flag} must set Health::verbose to true")
            }
            other => panic!("expected Command::Health, got {other:?}"),
        }
    }
}

#[test]
fn test_health_cli_default_verbose_is_false() {
    use clap::Parser;
    use geico::cli::{Cli, Command};

    let cli = Cli::try_parse_from(["geico", "health"]).expect("plain health must parse");
    match cli.command {
        Command::Health(args) => assert!(
            !args.verbose,
            "verbose must default to false without the flag"
        ),
        other => panic!("expected Command::Health, got {other:?}"),
    }
}

#[test]
fn test_health_sandbox_group_lists_available_backend() {
    let mut report = supported_report();
    report.sandbox_tools = vec![tool("bwrap", true), tool("firejail", false)];
    let text = report.format_text_with(false);
    assert!(
        text.contains("Sandbox backends:\n  [ok] bwrap\n"),
        "expected Sandbox backends with bwrap in:\n{text}"
    );
}
