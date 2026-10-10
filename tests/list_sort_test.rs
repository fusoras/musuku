//! Ordering behavior of `musu list`: default name ordering and the
//! `--sort` variants across the simple, `--json`, and `--ids` views.

use musu::manager::{ListOptions, Manager, SortField};
use musu::models::{AppEntry, AppRegistry};
use std::path::PathBuf;

/// Fixed timestamps so the `created` ordering is deterministic.
fn fixed(timestamp: &str) -> chrono::DateTime<chrono::Utc> {
    timestamp.parse().unwrap()
}

/// Build one entry with fully controlled `name`, `id`, `created_at`, and
/// `categories`.
fn entry(name: &str, id: &str, created: &str, categories: &[&str]) -> AppEntry {
    let mut app = AppEntry::new(Some(name), PathBuf::from(format!("/opt/portable/{id}")));
    app.id = id.to_string();
    app.created_at = fixed(created);
    app.categories = Some(categories.iter().map(|c| c.to_string()).collect());
    app
}

/// Registry seeded in a deliberately unsorted insertion order:
///
/// | name   | id       | created    | first category |
/// |--------|----------|------------|----------------|
/// | zed    | c-one    | 2020-01-01 | Video          |
/// | banana | a-two    | 2022-06-15 | Development    |
/// | Cherry | d-three  | 2021-03-10 | Audio          |
/// | Apple  | b-four   | 2023-12-01 | Utility        |
fn seeded_manager() -> (tempfile::TempDir, Manager) {
    let apps = vec![
        entry("zed", "c-one", "2020-01-01T00:00:00Z", &["Video"]),
        entry("banana", "a-two", "2022-06-15T00:00:00Z", &["Development"]),
        entry(
            "Cherry",
            "d-three",
            "2021-03-10T00:00:00Z",
            &["Audio", "Utility"],
        ),
        entry("Apple", "b-four", "2023-12-01T00:00:00Z", &["Utility"]),
    ];
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config/apps.toml");
    musu::config::save(&config_path, &AppRegistry { apps }).unwrap();
    let manager = Manager::new(
        config_path,
        temp.path().join("share"),
        temp.path().join("bin"),
    );
    (temp, manager)
}

/// Assert every name in `expected` appears in `output` in that relative order.
fn assert_order(output: &str, expected: &[&str]) {
    let mut previous: Option<(usize, &str)> = None;
    for name in expected {
        let index = output
            .find(name)
            .unwrap_or_else(|| panic!("{name:?} missing from output:\n{output}"));
        if let Some((previous_index, previous_name)) = previous {
            assert!(
                previous_index < index,
                "expected {previous_name:?} before {name:?} in output:\n{output}"
            );
        }
        previous = Some((index, name));
    }
}

fn list(manager: &Manager, options: ListOptions) -> String {
    manager.list_with_options(options).unwrap()
}

#[test]
fn test_list_default_sort_is_name_ascending_case_insensitive() {
    let (_temp, manager) = seeded_manager();
    let output = list(&manager, ListOptions::default());
    // Byte order would be Apple, Cherry, banana, zed; case-insensitive
    // ordering puts banana before Cherry.
    assert_order(&output, &["Apple", "banana", "Cherry", "zed"]);
}

#[test]
fn test_list_sort_name_ascending_explicit_matches_default() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::Name,
            ..Default::default()
        },
    );
    assert_order(&output, &["Apple", "banana", "Cherry", "zed"]);
}

#[test]
fn test_list_sort_name_desc_reverses_order() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::NameDesc,
            ..Default::default()
        },
    );
    assert_order(&output, &["zed", "Cherry", "banana", "Apple"]);
}

#[test]
fn test_list_sort_by_id_ascending() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::Id,
            ..Default::default()
        },
    );
    assert_order(&output, &["banana", "Apple", "zed", "Cherry"]);
    assert_order(&output, &["a-two", "b-four", "c-one", "d-three"]);
}

#[test]
fn test_list_sort_by_id_descending() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::IdDesc,
            ..Default::default()
        },
    );
    assert_order(&output, &["d-three", "c-one", "b-four", "a-two"]);
}

#[test]
fn test_list_sort_created_newest_first() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::Created,
            ..Default::default()
        },
    );
    assert_order(&output, &["Apple", "banana", "Cherry", "zed"]);
}

#[test]
fn test_list_sort_by_first_category_ascending() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            sort: SortField::Category,
            ..Default::default()
        },
    );
    assert_order(&output, &["Cherry", "banana", "Apple", "zed"]);
}

#[test]
fn test_list_json_output_is_sorted_by_name() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            json: true,
            ..Default::default()
        },
    );
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let names: Vec<&str> = parsed
        .as_array()
        .expect("JSON output must be an array")
        .iter()
        .map(|app| app["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Apple", "banana", "Cherry", "zed"]);
}

#[test]
fn test_list_ids_output_is_sorted_by_name() {
    let (_temp, manager) = seeded_manager();
    let output = list(
        &manager,
        ListOptions {
            ids: true,
            ..Default::default()
        },
    );
    let ids: Vec<&str> = output.lines().map(str::trim).collect();
    assert_eq!(ids, vec!["b-four", "a-two", "d-three", "c-one"]);
}
