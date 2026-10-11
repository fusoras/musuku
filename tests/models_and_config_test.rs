//! Milestone 1 integration tests: models (`AppEntry`/`AppRegistry`),
//! config (`apps.toml` load/save) and ANSI color constants.
//!
//! TDD protocol (`.agents/skills/test-validator/SKILL.md`):
//! every test in this file was executed BEFORE the implementation existed and
//! failed on its first run (RED), then passed once the code was written (GREEN).
//!
//! Isolation: all filesystem operations happen inside `tempfile::tempdir()`
//! workspaces, so the real user config (`~/.config/musu/apps.toml`) is never
//! read or written by tests.

use musu::colors;
use musu::config;
use musu::models::{AppEntry, AppRegistry, fallback_name, slugify};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// ----------------------------------------------------------------- colors ---

#[test]
fn test_colors_constants_are_exact_ansi_escape_codes() {
    assert_eq!(colors::BOLD_GREEN, "\x1b[1;32m");
    assert_eq!(colors::BOLD_BLUE, "\x1b[1;34m");
    assert_eq!(colors::BOLD_YELLOW, "\x1b[1;33m");
    assert_eq!(colors::BOLD_RED, "\x1b[1;31m");
    assert_eq!(colors::BOLD_CYAN, "\x1b[1;36m");
    assert_eq!(colors::DIM_GRAY, "\x1b[90m");
    assert_eq!(colors::RESET, "\x1b[0m");
}

// ----------------------------------------------------------------- models ---

#[test]
fn test_model_fallback_name_derived_from_target_bin_filename() {
    assert_eq!(
        fallback_name(Path::new("/path/to/blender-4.2/blender")),
        "blender"
    );
    assert_eq!(
        fallback_name(Path::new("/home/user/Downloads/Firefox.AppImage")),
        "Firefox"
    );
    assert_eq!(
        fallback_name(Path::new("/home/user/Downloads/kdenlive.appimage")),
        "kdenlive"
    );

    let entry = AppEntry::new(None, PathBuf::from("/opt/tools/foo"));
    assert_eq!(
        entry.name, "foo",
        "name must fall back to the target binary filename when not passed"
    );
    assert_eq!(entry.target_bin, PathBuf::from("/opt/tools/foo"));
}

#[test]
fn test_model_explicit_name_overrides_fallback_and_slugs_id() {
    let entry = AppEntry::new(Some("My Cool App!"), PathBuf::from("/tmp/tool"));
    assert_eq!(
        entry.name, "My Cool App!",
        "an explicit name must never be replaced by the fallback"
    );
    assert_eq!(entry.id, "my-cool-app", "id must be the slug of the name");

    assert_eq!(slugify("Foo Bar 2.0"), "foo-bar-2-0");
}

#[test]
fn test_model_new_entry_defaults_optional_fields_to_none() {
    let entry = AppEntry::new(None, PathBuf::from("/bin/bar"));
    assert_eq!(entry.id, "bar");
    assert!(entry.desktop_file.is_none());
    assert!(entry.symlink.is_none());
    assert!(entry.icon_path.is_none());
    assert!(entry.categories.is_none());

    let now = chrono::Utc::now();
    assert!(
        entry.created_at <= now + chrono::Duration::seconds(5),
        "created_at must not be in the future"
    );
    assert!(
        entry.created_at >= now - chrono::Duration::seconds(5),
        "created_at must be a fresh timestamp"
    );
}

#[test]
fn test_model_app_registry_add_and_find_by_id_or_name() {
    let mut registry = AppRegistry::new();
    assert!(registry.apps.is_empty());

    let alpha = AppEntry::new(Some("Alpha Tool"), PathBuf::from("/usr/bin/alpha"));
    let alpha_id = alpha.id.clone();
    registry.add(alpha);
    registry.add(AppEntry::new(None, PathBuf::from("/opt/beta")));

    assert_eq!(registry.apps.len(), 2);
    assert_eq!(registry.find(&alpha_id).unwrap().name, "Alpha Tool");
    assert!(
        registry.find("ALPHA TOOL").is_some(),
        "find must match names case-insensitively"
    );
    assert_eq!(
        registry.find("beta").unwrap().target_bin,
        PathBuf::from("/opt/beta")
    );
    assert!(registry.find("missing").is_none());
}

#[test]
fn test_model_app_registry_update_mutates_matched_entry() {
    let mut registry = AppRegistry::new();
    let entry = AppEntry::new(Some("Old Name"), PathBuf::from("/old/path"));
    let id = entry.id.clone();
    registry.add(entry);

    let new_target = PathBuf::from("/new/path");
    {
        let updated = registry
            .update(&id, |app| {
                app.name = "New Name".to_string();
                app.target_bin = new_target.clone();
            })
            .expect("entry must exist for update");
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.target_bin, new_target);
    }

    assert_eq!(
        registry.apps.len(),
        1,
        "update must not add or remove entries"
    );
    assert!(
        registry
            .update("nope", |app| app.name = "x".to_string())
            .is_none(),
        "updating an unknown entry must return None"
    );
}

#[test]
fn test_model_app_registry_remove_returns_entry_and_is_non_destructive() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    std::fs::write(&target, "#!/bin/sh\necho hi").unwrap();

    let mut registry = AppRegistry::new();
    registry.add(AppEntry::new(Some("My App"), target.clone()));
    registry.add(AppEntry::new(Some("Keeper"), temp.path().join("keeper")));

    let removed = registry.remove("my-app").expect("entry must be removable");
    assert_eq!(removed.name, "My App");
    assert_eq!(registry.apps.len(), 1);
    assert!(registry.find("my-app").is_none());
    assert!(
        registry.find("keeper").is_some(),
        "removal must only affect the matched entry"
    );
    assert!(
        target.exists(),
        "CRITICAL: Target binary must never be deleted!"
    );
    assert!(registry.remove("missing").is_none());
}

// ----------------------------------------------------------------- config ---

#[test]
fn test_config_default_path_targets_user_musu_directory() {
    let base = config::default_base_dir();
    assert_eq!(
        base.file_name().unwrap(),
        "musu",
        "default base dir must be the musu config directory"
    );

    let path = config::default_config_path();
    assert_eq!(path, base.join("apps.toml"));
    assert_eq!(path.file_name().unwrap(), "apps.toml");
}

#[test]
fn test_config_custom_base_dir_isolates_registry_from_user_config() {
    let temp = tempdir().unwrap();
    let base = temp.path().join("musu");
    let path = config::config_path(&base);

    assert_eq!(path, base.join("apps.toml"));
    assert_ne!(
        path,
        config::default_config_path(),
        "custom base dirs must resolve away from the real user config"
    );
}

#[test]
fn test_config_save_and_load_roundtrip_preserves_registry() {
    let temp = tempdir().unwrap();
    let base = temp.path().join("musu");
    let path = config::config_path(&base);

    let mut registry = AppRegistry::new();
    let mut full = AppEntry::new(Some("Full Entry"), PathBuf::from("/opt/full"));
    full.desktop_file = Some(PathBuf::from(
        "/home/user/.local/share/applications/full.desktop",
    ));
    full.symlink = Some(PathBuf::from("/home/user/.local/bin/full"));
    full.icon_path = Some(PathBuf::from("/home/user/.local/share/icons/full.png"));
    full.categories = Some(vec!["Utility".to_string(), "Development".to_string()]);
    registry.add(full);
    registry.add(AppEntry::new(None, PathBuf::from("/opt/plain")));

    config::save(&path, &registry).expect("save must succeed");
    assert!(path.exists(), "save must create apps.toml");

    let loaded = config::load(&path).expect("load must succeed");
    assert_eq!(loaded, registry, "roundtrip must preserve every field");
}

#[test]
fn test_config_save_creates_missing_parent_directories() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("nested/deeper/musu/apps.toml");
    let registry = AppRegistry::new();

    config::save(&path, &registry).expect("save must create parent directories");
    assert!(path.exists());

    let loaded = config::load(&path).unwrap();
    assert!(loaded.apps.is_empty());
}

#[test]
fn test_config_load_returns_empty_registry_when_file_missing() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("does-not-exist.toml");

    let registry = config::load(&path).expect("a missing file must not be an error");
    assert!(registry.apps.is_empty());
}

#[test]
fn test_config_load_reports_malformed_toml_as_error() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("apps.toml");
    std::fs::write(&path, "this is [[ not valid toml").unwrap();

    assert!(
        config::load(&path).is_err(),
        "a malformed apps.toml must surface an error instead of silent data loss"
    );
}

#[test]
fn test_config_loads_all_phase_two_options() {
    let temp = tempdir().unwrap();
    let path = config::settings_path(temp.path());
    std::fs::write(
        &path,
        r#"
[display]
images = "never"
list_icon_size = "large"
show_icon_size = "compact"
paths = "absolute"
color = "never"
list_format = "table"

[defaults]
categories = ["Development", "Utility"]
auto_extract = false
update_desktop_db = false
dest_bin = "~/custom-bin"
symlink_appimages = false
"#,
    )
    .unwrap();

    let settings = config::load_settings(temp.path()).unwrap();
    assert_eq!(settings.display.icons, config::IconPolicy::Never);
    assert_eq!(settings.display.list_icon_size, config::ListIconSize::Large);
    assert_eq!(
        settings.display.show_icon_size,
        config::ShowIconSize::Compact
    );
    assert_eq!(settings.display.paths, config::PathPolicy::Absolute);
    assert_eq!(settings.display.color, config::ColorPolicy::Never);
    assert_eq!(settings.display.list_format, config::ListFormat::Table);
    assert_eq!(settings.defaults.categories, ["Development", "Utility"]);
    assert!(!settings.defaults.auto_extract);
    assert!(!settings.defaults.update_desktop_db);
    assert_eq!(settings.defaults.dest_bin, PathBuf::from("~/custom-bin"));
    assert!(!settings.defaults.symlink_appimages);
}

#[test]
fn test_config_display_icons_persists_and_serializes_with_icons_key() {
    let temp = tempdir().unwrap();
    let path = config::settings_path(temp.path());

    // 1. Should load modern `icons` key
    std::fs::write(
        &path,
        r#"
[display]
icons = "always"
"#,
    )
    .unwrap();

    let settings = config::load_settings(temp.path()).unwrap();
    assert_eq!(settings.display.icons, config::IconPolicy::Always);

    // 2. Serializing must write `icons = "..."`, not `images = "..."`
    let save_dir = tempdir().unwrap();
    config::save_settings(save_dir.path(), &settings).unwrap();
    let serialized = std::fs::read_to_string(config::settings_path(save_dir.path())).unwrap();
    assert!(
        serialized.contains("icons = \"always\""),
        "config serialization must use 'icons = ...', got:\n{serialized}"
    );
    assert!(
        !serialized.contains("\nimages ="),
        "config serialization must not use obsolete 'images = ...', got:\n{serialized}"
    );

    // 3. Backward compatibility: should still parse legacy `images` key as alias
    let legacy_dir = tempdir().unwrap();
    std::fs::write(
        config::settings_path(legacy_dir.path()),
        r#"
[display]
images = "never"
"#,
    )
    .unwrap();
    let legacy_settings = config::load_settings(legacy_dir.path()).unwrap();
    assert_eq!(legacy_settings.display.icons, config::IconPolicy::Never);
}

#[test]
fn test_config_display_hide_ok_options_load_and_serialize() {
    let temp = tempdir().unwrap();
    let path = config::settings_path(temp.path());

    std::fs::write(
        &path,
        r#"
[display]
hide_target_ok = true
hide_desktop_ok = true
hide_ok_status = true
"#,
    )
    .unwrap();

    let settings = config::load_settings(temp.path()).unwrap();
    assert!(settings.display.hide_target_ok);
    assert!(settings.display.hide_desktop_ok);
    assert!(settings.display.hide_ok_status);

    // Test alias hide_ok
    let alias_dir = tempdir().unwrap();
    std::fs::write(
        config::settings_path(alias_dir.path()),
        r#"
[display]
hide_ok = true
"#,
    )
    .unwrap();
    let alias_settings = config::load_settings(alias_dir.path()).unwrap();
    assert!(alias_settings.display.hide_ok_status);
}

#[test]
fn test_config_display_description_options_load_and_serialize() {
    let temp = tempdir().unwrap();
    let path = config::settings_path(temp.path());

    std::fs::write(
        &path,
        r#"
[display]
description = true
"#,
    )
    .unwrap();

    let settings = config::load_settings(temp.path()).unwrap();
    assert!(settings.display.description);

    // Test alias show_description
    let alias_dir = tempdir().unwrap();
    std::fs::write(
        config::settings_path(alias_dir.path()),
        r#"
[display]
show_description = true
"#,
    )
    .unwrap();
    let alias_settings = config::load_settings(alias_dir.path()).unwrap();
    assert!(alias_settings.display.description);
}

// ------------------------------------------------------- sandbox profiles ---

/// `[sandbox]` holds one profile per app type; unknown types must parse (and
/// be detectable) instead of breaking the load, and the whole thing must
/// roundtrip through `save_settings`.
#[test]
fn test_config_sandbox_profiles_parse_and_roundtrip() {
    let temp = tempdir().unwrap();
    std::fs::write(
        config::settings_path(temp.path()),
        r#"
[sandbox.appimage]
enabled = true
backend = "bwrap"
network = false

[sandbox.foobar]
enabled = true
"#,
    )
    .unwrap();

    let settings = config::load_settings(temp.path()).unwrap();
    let profile = settings
        .sandbox
        .profile_for_type("appimage")
        .expect("the appimage profile must resolve");
    assert_eq!(profile.enabled, Some(true));
    assert_eq!(profile.backend, "bwrap");
    assert_eq!(profile.network, Some(false));

    // Unknown types keep parsing but never resolve as a profile.
    assert!(settings.sandbox.profiles.extra.contains_key("foobar"));
    assert!(
        settings.sandbox.profile_for_type("foobar").is_none(),
        "unsupported types must not resolve to a profile"
    );

    // Roundtrip: profiles serialize under `[sandbox.<type>]`, never as flat
    // keys or a nested `[sandbox.policy]` level.
    let save_dir = tempdir().unwrap();
    config::save_settings(save_dir.path(), &settings).unwrap();
    let text = std::fs::read_to_string(config::settings_path(save_dir.path())).unwrap();
    assert!(text.contains("[sandbox.appimage]"), "got:\n{text}");
    assert!(text.contains("[sandbox.foobar]"), "got:\n{text}");
    assert!(!text.contains("[sandbox.policy]"), "got:\n{text}");
    let reloaded = config::load_settings(save_dir.path()).unwrap();
    assert_eq!(reloaded.sandbox, settings.sandbox);
}

/// The pre-profile flat format is rejected with a migration hint instead of
/// being silently reinterpreted.
#[test]
fn test_config_rejects_legacy_flat_sandbox_format() {
    let temp = tempdir().unwrap();
    std::fs::write(
        config::settings_path(temp.path()),
        "[sandbox]\nenabled = true\nbackend = \"bwrap\"\n",
    )
    .unwrap();

    let err = config::load_settings(temp.path()).unwrap_err();
    let message = format!("{err:#}");
    assert!(
        message.contains("legacy [sandbox] format detected"),
        "got: {message}"
    );
    assert!(
        message.contains("migrate to per-type profiles"),
        "got: {message}"
    );
    assert!(
        message.contains("config.toml"),
        "the error must name the offending file: {message}"
    );
    assert!(
        message.contains("[sandbox.appimage]"),
        "the error must include the migration snippet: {message}"
    );
}

/// The legacy `[sandbox.policy]` level is equally rejected.
#[test]
fn test_config_rejects_legacy_sandbox_policy_section() {
    let temp = tempdir().unwrap();
    std::fs::write(
        config::settings_path(temp.path()),
        "[sandbox]\nenabled = true\n[sandbox.policy]\nnetwork = false\n",
    )
    .unwrap();

    let err = config::load_settings(temp.path()).unwrap_err();
    let message = format!("{err:#}");
    assert!(
        message.contains("migrate to per-type profiles"),
        "got: {message}"
    );
}

/// A `[sandbox]` section that only contains profiles (or nothing at all)
/// still loads: only the legacy flat keys are rejected.
#[test]
fn test_config_accepts_empty_and_profile_only_sandbox_sections() {
    let empty = tempdir().unwrap();
    std::fs::write(config::settings_path(empty.path()), "[sandbox]\n").unwrap();
    let settings = config::load_settings(empty.path()).unwrap();
    assert!(settings.sandbox.profiles.appimage.is_none());

    let defaults = tempdir().unwrap();
    std::fs::write(
        config::settings_path(defaults.path()),
        "[display]\ncolor = \"never\"\n",
    )
    .unwrap();
    let settings = config::load_settings(defaults.path()).unwrap();
    assert_eq!(settings.sandbox, config::SandboxConfig::default());
}
