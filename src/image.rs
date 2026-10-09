use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const KITTY_CHUNK_SIZE: usize = 4096;

pub fn encode_base64(data: &[u8]) -> String {
    let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied();
        let third = chunk.get(2).copied();

        encoded.push(BASE64_ALPHABET[(first >> 2) as usize] as char);
        encoded.push(
            BASE64_ALPHABET[((first & 0x03) << 4 | second.unwrap_or(0) >> 4) as usize] as char,
        );
        encoded.push(match second {
            Some(value) => {
                BASE64_ALPHABET[((value & 0x0f) << 2 | third.unwrap_or(0) >> 6) as usize] as char
            }
            None => '=',
        });
        encoded.push(match third {
            Some(value) => BASE64_ALPHABET[(value & 0x3f) as usize] as char,
            None => '=',
        });
    }

    encoded
}

pub fn is_kitty_compatible_terminal() -> bool {
    env::var_os("MUSU_IMAGE").is_some()
        || env::var_os("KITTY_WINDOW_ID").is_some()
        || env::var_os("KITTY_PID").is_some()
        || env::var_os("KITTY_INSTALLATION_DIR").is_some()
        || env::var_os("GHOSTTY_RESOURCES_DIR").is_some()
        || env::var("TERM")
            .map(|term| term.contains("kitty") || term.contains("ghostty"))
            .unwrap_or(false)
        || matches!(
            env::var("TERM_PROGRAM").as_deref(),
            Ok("WezTerm") | Ok("ghostty") | Ok("herdr")
        )
}

pub fn supports_kitty_graphics() -> bool {
    io::stdout().is_terminal() && is_kitty_compatible_terminal()
}

pub fn icons_requested(
    policy: crate::config::IconPolicy,
    explicit: bool,
    suppressed: bool,
) -> bool {
    if suppressed || policy == crate::config::IconPolicy::Never {
        return false;
    }
    explicit
        || policy == crate::config::IconPolicy::Always
        || (policy == crate::config::IconPolicy::Auto && supports_kitty_graphics())
}

pub use icons_requested as image_requested;

pub fn generate_kitty_escape(data: &[u8], cols: u32, rows: u32) -> String {
    if data.is_empty() {
        return String::new();
    }

    let chunks: Vec<String> = data.chunks(KITTY_CHUNK_SIZE).map(encode_base64).collect();
    let mut escape = String::new();

    for (index, chunk) in chunks.iter().enumerate() {
        let continuation = usize::from(index + 1 < chunks.len());
        escape.push_str(&format!(
            "\x1b_Ga=T,f=100,c={cols},r={rows},m={continuation};{chunk}\x1b\\"
        ));
    }

    escape
}

pub fn rasterize_svg_if_needed(path: &Path) -> Option<PathBuf> {
    if !path.extension()?.to_str()?.eq_ignore_ascii_case("svg") || !path.is_file() {
        return None;
    }

    if path.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        return None;
    }

    // Converter priority: specific tool first, general-purpose fallbacks after.
    // 1. rsvg-convert (librsvg): purpose-built SVG renderer, fastest startup
    //    and matches GNOME's rendering (same library).
    // 2. magick / convert (ImageMagick 7 / 6): heavier startup and their SVG
    //    output depends on delegates and policy.xml, which may block SVG.
    // Without any of them the preview is skipped silently; the desktop
    // launcher is unaffected because .desktop icons are copied as-is.
    let tool = ["rsvg-convert", "magick", "convert"]
        .into_iter()
        .find_map(find_in_path)?;
    let output = env::temp_dir().join(format!("musu-icon-{}.png", unique_suffix()));

    let status = if tool.file_name().and_then(|name| name.to_str()) == Some("rsvg-convert") {
        Command::new(&tool)
            .args(["-o", output.to_str()?, path.to_str()?])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new(&tool)
            .args([path.to_str()?, output.to_str()?])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    }
    .ok()?;

    if status.success()
        && output.is_file()
        && output.metadata().map(|m| m.len() > 0).unwrap_or(false)
    {
        Some(output)
    } else {
        let _ = fs::remove_file(output);
        None
    }
}

pub fn render_icon_preview(icon_path: &Path, cols: u32, rows: u32, force: bool) -> Option<String> {
    if (!force && !supports_kitty_graphics()) || !icon_path.is_file() {
        return None;
    }

    if icon_path.metadata().map(|m| m.len() == 0).unwrap_or(true) {
        return None;
    }

    let is_svg = icon_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("svg"))
        .unwrap_or(false);

    let rasterized = if is_svg {
        Some(rasterize_svg_if_needed(icon_path)?)
    } else {
        None
    };

    let image_path = rasterized.as_deref().unwrap_or(icon_path);
    let data = fs::read(image_path).ok()?;
    if let Some(path) = rasterized {
        let _ = fs::remove_file(path);
    }

    if data.is_empty() {
        return None;
    }

    Some(generate_kitty_escape(&data, cols, rows))
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    env::split_paths(&env::var_os("PATH")?)
        .map(|directory| directory.join(command))
        .find(|path| {
            path.is_file()
                && path
                    .metadata()
                    .map(|metadata| {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            metadata.permissions().mode() & 0o111 != 0
                        }
                        #[cfg(not(unix))]
                        {
                            !metadata.permissions().readonly()
                        }
                    })
                    .unwrap_or(false)
        })
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}
