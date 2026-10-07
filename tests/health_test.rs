use geico::health::{HealthReport, RegistrySummary, detect_terminal_graphics};
use geico::models::AppRegistry;
use std::path::PathBuf;

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
