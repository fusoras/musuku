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

#[test]
fn test_extractor_ignores_non_image_files_with_matching_stem() {
    let temp = tempdir().unwrap();
    let root = temp.path();

    // Create an XML file with matching stem in a subdirectory
    let mime_dir = root.join("usr/share/mime");
    fs::create_dir_all(&mime_dir).unwrap();
    let xml_file = mime_dir.join("org.shotcut.Shotcut.xml");
    fs::write(&xml_file, "<mime-info></mime-info>").unwrap();

    // Ensure find_icon doesn't return the XML file
    let found = geico::extractor::find_icon(root, Some("org.shotcut.Shotcut"));
    assert_ne!(
        found,
        Some(xml_file),
        "find_icon should never pick an XML file as an icon"
    );

    // Now add a valid SVG icon and ensure it is selected
    let svg_file = root.join("org.shotcut.Shotcut.svg");
    fs::write(&svg_file, "<svg></svg>").unwrap();
    let found_svg = geico::extractor::find_icon(root, Some("org.shotcut.Shotcut"));
    assert_eq!(found_svg, Some(svg_file));
}

#[test]
fn test_extractor_drop_cleans_up_temp_dir() {
    let temp_parent = tempdir().unwrap();
    let temp_root = temp_parent.path().join("geico-appimage-test-temp");
    fs::create_dir_all(&temp_root).unwrap();
    assert!(temp_root.exists());

    {
        let _extraction = geico::extractor::Extraction {
            metadata: DesktopMetadata::default(),
            icon_path: None,
            root: temp_root.clone(),
        };
    }

    assert!(
        !temp_root.exists(),
        "Extraction::drop must automatically clean up its temporary root directory"
    );
}
