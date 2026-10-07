use geico::health::{HealthReport, RegistrySummary, detect_terminal_graphics};
use geico::models::AppRegistry;
use std::path::PathBuf;

fn supported_report() -> HealthReport {
    HealthReport {
        terminal_graphics: detect_terminal_graphics(Some("ghostty"), None),
        svg_tools: vec![],
        desktop_tools: vec![],
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
    let text = supported_report().format_text();
    assert!(text.contains('✅'), "expected ✅ marker in:\n{text}");
    assert!(text.contains("OK"), "expected OK status word in:\n{text}");
    assert!(
        text.contains("kitty graphics protocol"),
        "expected kitty graphics protocol wording in:\n{text}"
    );
    assert!(
        !text.contains("Terminal graphics:"),
        "legacy 'Terminal graphics:' line must be replaced in:\n{text}"
    );
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
    let text = unsupported_report().format_text();
    assert!(text.contains('⚠'), "expected ⚠ marker in:\n{text}");
    assert!(
        text.contains("WARNING"),
        "expected WARNING status word in:\n{text}"
    );
    assert!(
        text.contains("kitty graphics protocol"),
        "expected kitty graphics protocol wording in:\n{text}"
    );
    assert!(
        !text.contains("xterm"),
        "detected terminal name must not be printed in:\n{text}"
    );
    assert!(
        !text.contains("Terminal graphics:"),
        "legacy 'Terminal graphics:' line must be replaced in:\n{text}"
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
            .any(|line| line == "✅ OK your terminal supports the kitty graphics protocol"),
        "expected exact supported literal in:\n{supported}"
    );
    let unsupported = unsupported_report().format_text_with(false);
    assert!(
        unsupported
            .lines()
            .any(|line| line
                == "⚠ WARNING your terminal does not support the kitty graphics protocol"),
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
        supported.contains("\x1b[1;32m✅ OK\x1b[0m"),
        "expected green ✅ OK in:\n{supported:?}"
    );
    let unsupported = unsupported_report().format_text_with(true);
    assert!(
        unsupported.contains("\x1b[1;33m⚠ WARNING\x1b[0m"),
        "expected yellow ⚠ WARNING in:\n{unsupported:?}"
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
