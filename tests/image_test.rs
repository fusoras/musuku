use geico::image::{
    encode_base64, generate_kitty_escape, rasterize_svg_if_needed, render_icon_preview,
    supports_kitty_graphics,
};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_base64_encode_rfc4648() {
    assert_eq!(encode_base64(b""), "");
    assert_eq!(encode_base64(b"f"), "Zg==");
    assert_eq!(encode_base64(b"fo"), "Zm8=");
    assert_eq!(encode_base64(b"foo"), "Zm9v");
    assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn test_kitty_escape_format() {
    let escape = generate_kitty_escape(b"hello", 2, 3);

    assert_eq!(escape, "\x1b_Ga=T,f=100,c=2,r=3,m=0;aGVsbG8=\x1b\\");
}

#[test]
fn test_supports_kitty_graphics() {
    let _ = supports_kitty_graphics();
}

#[test]
fn test_render_icon_preview_raster_and_svg() {
    let temp = tempdir().unwrap();
    let png = temp.path().join("icon.png");
    std::fs::write(&png, b"not a real png").unwrap();

    assert!(render_icon_preview(&png, 1, 1).is_none() || supports_kitty_graphics());

    let svg = temp.path().join("icon.svg");
    std::fs::write(&svg, "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").unwrap();
    if let Some(converted) = rasterize_svg_if_needed(Path::new(&svg)) {
        assert_eq!(
            converted.extension().and_then(|ext| ext.to_str()),
            Some("png")
        );
        assert!(converted.exists());
        let _ = std::fs::remove_file(converted);
    }
}
