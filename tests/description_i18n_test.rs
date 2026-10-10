//! Tests for multilingual descriptions (`--description LANG=text`).
//!
//! Covers the `lang=text` CLI parsing, localized `Comment[lang]=` lines in
//! the generated `.desktop` file, per-language merge semantics on update,
//! registry/bundle persistence, and extraction of `Comment[lang]` keys from
//! AppImage metadata.

use musu::extractor::parse_desktop_entry;
use musu::manager::{AddOptions, Manager, ShowOptions, UpdateOptions, parse_description_values};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

fn executable(path: &std::path::Path) {
    fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn manager_with(base: &std::path::Path) -> Manager {
    Manager::new(
        base.join("config/apps.toml"),
        base.join("share"),
        base.join("bin"),
    )
}

fn langs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(lang, text)| (lang.to_string(), text.to_string()))
        .collect()
}

#[test]
fn test_parse_description_values_plain_default_only() {
    let (default, languages) =
        parse_description_values(vec!["A simple description".into()]).unwrap();
    assert_eq!(default.as_deref(), Some("A simple description"));
    assert_eq!(languages, None);
}

#[test]
fn test_parse_description_values_separates_default_and_languages() {
    let (default, languages) = parse_description_values(vec![
        "Default text".into(),
        "es=Texto en español".into(),
        "fr=Texte en français".into(),
    ])
    .unwrap();
    assert_eq!(default.as_deref(), Some("Default text"));
    assert_eq!(
        languages,
        Some(langs(&[
            ("es", "Texto en español"),
            ("fr", "Texte en français"),
        ]))
    );
}

#[test]
fn test_parse_description_values_languages_only() {
    let (default, languages) = parse_description_values(vec!["pt_BR=Descrição".into()]).unwrap();
    assert_eq!(default, None);
    assert_eq!(languages, Some(langs(&[("pt_BR", "Descrição")])));
}

#[test]
fn test_parse_description_values_invalid_lang_is_treated_as_plain_text() {
    // Codes that don't look like locales (`--`, digits-first, too short/long)
    // must not be swallowed as languages.
    let (default, languages) = parse_description_values(vec![
        "1nvalid=x".into(),
        "-=y".into(),
        "verylonglanguagecode=z".into(),
    ])
    .unwrap();
    assert_eq!(default.as_deref(), Some("verylonglanguagecode=z"));
    assert_eq!(languages, None);
}

#[test]
fn test_parse_description_values_empty_language_value_is_ignored() {
    let (default, languages) =
        parse_description_values(vec!["es=".into(), "Plain".into()]).unwrap();
    assert_eq!(default.as_deref(), Some("Plain"));
    assert_eq!(languages, None);
}

#[test]
fn test_parse_desktop_entry_extracts_comment_langs() {
    let metadata = parse_desktop_entry(
        "[Desktop Entry]\nName=App\nComment=Default comment\nComment[es]=Comentario\nComment[fr]=Commentaire\nComment[zz]=\n",
    )
    .unwrap();
    assert_eq!(metadata.comment, Some("Default comment".into()));
    assert_eq!(
        metadata.comment_langs,
        langs(&[("es", "Comentario"), ("fr", "Commentaire")])
    );
}

#[test]
fn test_add_writes_localized_comments_to_desktop_file() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = manager_with(temp.path());

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            description: Some("Default text".into()),
            descriptions: Some(langs(&[
                ("es", "Texto en español"),
                ("fr", "Texte en français"),
            ])),
            ..Default::default()
        })
        .unwrap();

    let desktop_content = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        desktop_content.contains("Comment=Default text\n"),
        "expected default Comment, got:\n{desktop_content}"
    );
    assert!(
        desktop_content.contains("Comment[es]=Texto en español\n"),
        "expected localized es Comment, got:\n{desktop_content}"
    );
    assert!(
        desktop_content.contains("Comment[fr]=Texte en français\n"),
        "expected localized fr Comment, got:\n{desktop_content}"
    );
}

#[test]
fn test_add_without_default_description_writes_only_localized_comments() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = manager_with(temp.path());

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            descriptions: Some(langs(&[("es", "Solo en español")])),
            ..Default::default()
        })
        .unwrap();

    let desktop_content = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(
        !desktop_content.contains("Comment="),
        "no default Comment expected, got:\n{desktop_content}"
    );
    assert!(desktop_content.contains("Comment[es]=Solo en español\n"));
}

#[test]
fn test_descriptions_persist_in_registry_roundtrip() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = manager_with(temp.path());

    manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            descriptions: Some(langs(&[("es", "Persistido")])),
            ..Default::default()
        })
        .unwrap();

    // A freshly loaded registry must keep the per-language descriptions.
    let loaded = musu::config::load(&temp.path().join("config/apps.toml")).unwrap();
    let app = loaded.find("MyApp").unwrap();
    assert_eq!(app.descriptions, Some(langs(&[("es", "Persistido")])));
}

#[test]
fn test_show_displays_language_descriptions() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = manager_with(temp.path());

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            description: Some("Default text".into()),
            descriptions: Some(langs(&[("es", "Texto en español")])),
            ..Default::default()
        })
        .unwrap();

    let show_output = manager.show(&entry.id, ShowOptions::default()).unwrap();
    assert!(show_output.contains("Description: Default text"));
    assert!(
        show_output.contains("[es]: Texto en español"),
        "expected localized description in show output, got:\n{show_output}"
    );
}

#[test]
fn test_update_merges_per_language_descriptions() {
    let temp = tempdir().unwrap();
    let target = temp.path().join("my-app");
    executable(&target);
    let manager = manager_with(temp.path());

    let entry = manager
        .add(AddOptions {
            target: target.clone(),
            name: Some("MyApp".into()),
            descriptions: Some(langs(&[("es", "Español"), ("fr", "Français")])),
            ..Default::default()
        })
        .unwrap();

    // Updating `es` and adding `de` must keep `fr` untouched.
    manager
        .update(
            &entry.id,
            UpdateOptions {
                descriptions: Some(langs(&[("es", "Español actualizado"), ("de", "Deutsch")])),
                ..Default::default()
            },
        )
        .unwrap();

    let updated = manager.get(&entry.id).unwrap();
    assert_eq!(
        updated.descriptions,
        Some(langs(&[
            ("de", "Deutsch"),
            ("es", "Español actualizado"),
            ("fr", "Français"),
        ]))
    );

    // An empty value removes just that language.
    manager
        .update(
            &entry.id,
            UpdateOptions {
                descriptions: Some(langs(&[("fr", "")])),
                ..Default::default()
            },
        )
        .unwrap();

    let updated = manager.get(&entry.id).unwrap();
    assert_eq!(
        updated.descriptions,
        Some(langs(&[("de", "Deutsch"), ("es", "Español actualizado")]))
    );

    let desktop_content = fs::read_to_string(entry.desktop_file.as_ref().unwrap()).unwrap();
    assert!(desktop_content.contains("Comment[es]=Español actualizado\n"));
    assert!(desktop_content.contains("Comment[de]=Deutsch\n"));
    assert!(
        !desktop_content.contains("Comment[fr]="),
        "removed language must not remain in desktop file, got:\n{desktop_content}"
    );
}

#[test]
fn test_bundle_roundtrip_preserves_descriptions() {
    let temp = tempdir().unwrap();
    let app_dir = temp.path().join("app");
    fs::create_dir_all(&app_dir).unwrap();
    let bin_path = app_dir.join("main.sh");
    executable(&bin_path);
    let manager = manager_with(temp.path());

    manager
        .add(AddOptions {
            target: bin_path.clone(),
            name: Some("MyApp".into()),
            descriptions: Some(langs(&[("es", "Del bundle")])),
            portable: true,
            ..Default::default()
        })
        .unwrap();

    let bundle = musu::bundle::load_bundle(&app_dir.join(".musu.toml")).unwrap();
    assert_eq!(
        bundle.app.descriptions,
        Some(langs(&[("es", "Del bundle")]))
    );
}

#[test]
fn test_cli_parses_repeated_description_flag() {
    use clap::Parser;
    let cli = musu::cli::Cli::parse_from([
        "musu",
        "add",
        "./app",
        "--description",
        "Default text",
        "--description",
        "es=Texto en español",
        "-d",
        "fr=Texte en français",
    ]);
    let musu::cli::Command::Add(args) = cli.command else {
        panic!("expected add command");
    };
    assert_eq!(
        args.description,
        vec![
            "Default text".to_string(),
            "es=Texto en español".to_string(),
            "fr=Texte en français".to_string(),
        ]
    );
}
