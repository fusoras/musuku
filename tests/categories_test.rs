use geico::categories::{
    CATEGORY_NAMES, STANDARD_CATEGORIES, format_categories_table, normalize_category,
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
