use geico::extractor::{DesktopMetadata, inspect_and_extract, parse_desktop_entry};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;

#[test]
fn test_extractor_infers_desktop_metadata() {
    let metadata = parse_desktop_entry(
        "[Desktop Entry]\nName=Video Tool\nComment=Play videos\nCategories=AudioVideo;Utility;\nIcon=video-tool\n",
    )
    .unwrap();
    assert_eq!(
        metadata,
        DesktopMetadata {
            name: Some("Video Tool".into()),
            comment: Some("Play videos".into()),
            categories: Some(vec!["AudioVideo".into(), "Utility".into()]),
            icon: Some("video-tool".into()),
        }
    );
}

#[test]
fn test_extractor_skips_non_appimage_files() {
    let temp = tempdir().unwrap();
    let script = temp.path().join("plain-script");
    fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(inspect_and_extract(&script).unwrap().is_none());
}
