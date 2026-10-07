use geico::categories::{
    CATEGORY_NAMES, STANDARD_CATEGORIES, format_categories_table, is_known_category,
    normalize_category, render_categories_query, suggest_category, unknown_category_warning,
};

#[test]
fn test_normalize_category_exact_and_case_insensitive() {
    assert_eq!(normalize_category("development"), "Development");
    assert_eq!(normalize_category("AUDIOVIDEO"), "AudioVideo");
    assert_eq!(normalize_category("graphics"), "Graphics");
    assert_eq!(normalize_category("utility"), "Utility");
}

#[test]
fn test_normalize_category_fuzzy_and_aliases() {
    assert_eq!(normalize_category("dev"), "Development");
    assert_eq!(normalize_category("programacion"), "Development");
    assert_eq!(normalize_category("video"), "Video");
    assert_eq!(normalize_category("multimedia"), "AudioVideo");
    assert_eq!(normalize_category("graficos"), "Graphics");
    assert_eq!(normalize_category("util"), "Utility");
    assert_eq!(normalize_category("internet"), "Network");
    assert_eq!(normalize_category("juegos"), "Game");
}

#[test]
fn test_format_categories_table_contains_standard_names_and_examples() {
    let output = format_categories_table();
    assert!(!output.is_empty(), "Table output should not be empty");
    assert!(output.contains("AudioVideo"));
    assert!(output.contains("Shotcut"));
    assert!(output.contains("Development"));
    assert!(output.contains("VS Code"));
    assert_eq!(CATEGORY_NAMES.len(), STANDARD_CATEGORIES.len());
}

#[test]
fn test_is_known_category_accepts_standard_alias_prefix_and_rejects_typos() {
    for known in ["Video", "video", "dev", "programación", "mult"] {
        assert!(
            is_known_category(known),
            "'{known}' should resolve to a standard category"
        );
    }
    for unknown in ["juegoz", "", "   "] {
        assert!(
            !is_known_category(unknown),
            "'{unknown}' should fall back to a custom category"
        );
    }
}

#[test]
fn test_suggest_category_matches_close_standard_names_only() {
    assert_eq!(suggest_category("juegoz"), Some("Game"));
    assert_eq!(suggest_category("networ"), Some("Network"));
    assert_eq!(suggest_category("zzzzzzzzzz"), None);
    // Containment fallback: no close edit distance, but it contains "video".
    assert_eq!(suggest_category("videoeditor"), Some("Video"));
    assert_eq!(suggest_category(""), None);
}

#[test]
fn test_unknown_category_warning_only_for_unknown_values_and_respects_color() {
    let warning = unknown_category_warning("juegoz", false).expect("a typo must produce a warning");
    assert!(
        warning.contains("Unknown category 'juegoz' — kept as custom. Did you mean: Game?"),
        "got: {warning}"
    );
    assert!(
        warning.contains("Run 'geico update <app> --categories'"),
        "hint must point at the no-value query mode; got: {warning}"
    );
    assert!(
        !warning.contains("geico categories"),
        "hint must not reference the removed subcommand; got: {warning}"
    );
    assert!(
        !warning.contains('\u{1b}'),
        "color=false must render plain text; got: {warning:?}"
    );

    assert!(
        unknown_category_warning("Video", false).is_none(),
        "known categories must not warn"
    );
    assert!(
        unknown_category_warning("", false).is_none(),
        "empty values must not warn"
    );

    let colored = unknown_category_warning("juegoz", true).expect("a typo must warn");
    assert!(
        colored.contains('\u{1b}'),
        "color=true must go through the color helpers; got: {colored:?}"
    );
}

#[test]
fn test_render_categories_query_layout_for_update_and_add() {
    let update_tip = "Usage tip: geico update openshot --categories Video,AudioVideo";
    let update = render_categories_query(
        Some((
            "openshot",
            &["Video".to_string(), "AudioVideo".to_string()][..],
        )),
        update_tip,
    );
    assert!(
        update.starts_with("Current categories for 'openshot': Video,AudioVideo\n\n"),
        "header must be followed by a blank line; got: {update:?}"
    );
    assert!(update.contains("Standard FreeDesktop Application Categories"));
    assert!(
        update
            .trim_end()
            .lines()
            .next_back()
            .is_some_and(|line| line.contains(update_tip)),
        "the specific tip must be the last line; got: {update:?}"
    );

    let bare = render_categories_query(Some(("bare", &[][..])), "bare-tip");
    assert!(
        bare.contains("Current categories for 'bare': (none)"),
        "apps without categories must show (none); got: {bare:?}"
    );

    let add_tip = "Usage tip: geico add <path> --categories Video,AudioVideo";
    let add = render_categories_query(None, add_tip);
    assert!(
        !add.contains("Current categories"),
        "add mode must not print a current-categories header; got: {add:?}"
    );
    assert!(add.contains("Standard FreeDesktop Application Categories"));
    assert!(
        add.trim_end()
            .lines()
            .next_back()
            .is_some_and(|line| line.contains(add_tip)),
        "got: {add:?}"
    );
}
