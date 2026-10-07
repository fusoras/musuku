//! Predefined FreeDesktop desktop entry categories and smart normalization.

use crate::colors::{BOLD_BLUE, BOLD_CYAN, DIM_GRAY, RESET};

#[derive(Debug, Clone, Copy)]
pub struct CategoryInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub examples: &'static str,
}

pub const STANDARD_CATEGORIES: &[CategoryInfo] = &[
    CategoryInfo {
        name: "AudioVideo",
        description: "Multimedia, audio and video applications",
        examples: "VLC, Shotcut, Audacity, Spotify",
    },
    CategoryInfo {
        name: "Audio",
        description: "Audio players, recorders, and editors",
        examples: "Audacity, Spotify, Ardour",
    },
    CategoryInfo {
        name: "Video",
        description: "Video players, recorders, and editors",
        examples: "Shotcut, Kdenlive, OBS Studio",
    },
    CategoryInfo {
        name: "Development",
        description: "IDEs, text editors, programming tools",
        examples: "VS Code, RustRover, GitKraken",
    },
    CategoryInfo {
        name: "Education",
        description: "Educational and learning software",
        examples: "Anki, Stellarium, Tux Paint",
    },
    CategoryInfo {
        name: "Game",
        description: "Games and emulators",
        examples: "Steam, RetroArch, Ryujinx",
    },
    CategoryInfo {
        name: "Graphics",
        description: "Image viewers, raster/vector editors, 3D",
        examples: "Blender, GIMP, Inkscape, Krita",
    },
    CategoryInfo {
        name: "Network",
        description: "Web browsers, chat, email, remote tools",
        examples: "Firefox, Discord, Telegram, FileZilla",
    },
    CategoryInfo {
        name: "Office",
        description: "Office suites, document viewers, notes",
        examples: "LibreOffice, Obsidian, Foliate",
    },
    CategoryInfo {
        name: "Science",
        description: "Scientific software, mathematics",
        examples: "GeoGebra, Octave, QGIS",
    },
    CategoryInfo {
        name: "Settings",
        description: "Settings and configuration utilities",
        examples: "GNOME Tweaks, Pavucontrol",
    },
    CategoryInfo {
        name: "System",
        description: "System utilities, terminal emulators, monitors",
        examples: "Alacritty, GParted, Htop",
    },
    CategoryInfo {
        name: "Utility",
        description: "General accessories, archivers, calculators",
        examples: "PeaZip, Flameshot, Calculator",
    },
];

pub const CATEGORY_NAMES: &[&str] = &[
    "AudioVideo",
    "Audio",
    "Video",
    "Development",
    "Education",
    "Game",
    "Graphics",
    "Network",
    "Office",
    "Science",
    "Settings",
    "System",
    "Utility",
];

/// Common aliases and Spanish synonyms mapped to their standard category.
///
/// Single source of truth shared by [`normalize_category`] (exact match and
/// prefix matching) and [`suggest_category`] (did-you-mean hints).
pub const CATEGORY_ALIASES: &[(&str, &str)] = &[
    // Development
    ("dev", "Development"),
    ("programacion", "Development"),
    ("programación", "Development"),
    ("code", "Development"),
    ("coding", "Development"),
    ("ide", "Development"),
    // Video
    ("vid", "Video"),
    ("video", "Video"),
    ("pelicula", "Video"),
    ("peliculas", "Video"),
    // Audio
    ("musica", "Audio"),
    ("música", "Audio"),
    ("music", "Audio"),
    ("sonido", "Audio"),
    ("audio", "Audio"),
    // AudioVideo
    ("multimedia", "AudioVideo"),
    ("media", "AudioVideo"),
    // Graphics
    ("graficos", "Graphics"),
    ("gráficos", "Graphics"),
    ("grafico", "Graphics"),
    ("gráfico", "Graphics"),
    ("design", "Graphics"),
    ("diseno", "Graphics"),
    ("diseño", "Graphics"),
    ("art", "Graphics"),
    ("arte", "Graphics"),
    ("3d", "Graphics"),
    // Utility
    ("util", "Utility"),
    ("utils", "Utility"),
    ("utilidades", "Utility"),
    ("utilidad", "Utility"),
    ("tools", "Utility"),
    ("tool", "Utility"),
    ("herramienta", "Utility"),
    ("herramientas", "Utility"),
    // Network
    ("web", "Network"),
    ("internet", "Network"),
    ("browser", "Network"),
    ("navegador", "Network"),
    ("red", "Network"),
    ("redes", "Network"),
    // Game
    ("juegos", "Game"),
    ("juego", "Game"),
    ("games", "Game"),
    ("game", "Game"),
    // Office
    ("ofimatica", "Office"),
    ("ofimática", "Office"),
    ("documentos", "Office"),
    ("docs", "Office"),
    // Science
    ("ciencia", "Science"),
    ("ciencias", "Science"),
    ("science", "Science"),
    ("math", "Science"),
    ("matematicas", "Science"),
    ("matemáticas", "Science"),
    // Settings
    ("config", "Settings"),
    ("configuracion", "Settings"),
    ("configuración", "Settings"),
    ("ajustes", "Settings"),
    ("preferencias", "Settings"),
    // System
    ("sistema", "System"),
    ("sys", "System"),
    // Education
    ("educacion", "Education"),
    ("educación", "Education"),
    ("educativo", "Education"),
    ("learn", "Education"),
    ("learning", "Education"),
];

/// Smartly normalizes a category string to official FreeDesktop standard name.
/// Handles exact case-insensitive matches, common aliases, and prefix matching
/// (against both standard names and aliases).
pub fn normalize_category(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // 1. Direct case-insensitive match against standard categories
    for cat in STANDARD_CATEGORIES {
        if trimmed.eq_ignore_ascii_case(cat.name) {
            return cat.name.to_string();
        }
    }

    // 2. Common aliases and Spanish synonyms
    let lower = trimmed.to_ascii_lowercase();
    if let Some(canonical) = CATEGORY_ALIASES
        .iter()
        .find(|(alias, _)| *alias == lower.as_str())
        .map(|(_, canonical)| *canonical)
    {
        return canonical.to_string();
    }

    // 3. Prefix matching against standard names, then aliases (min length 3)
    if lower.len() >= 3 {
        for cat in STANDARD_CATEGORIES {
            if cat.name.to_ascii_lowercase().starts_with(&lower) {
                return cat.name.to_string();
            }
        }
        for (alias, canonical) in CATEGORY_ALIASES {
            if alias.starts_with(&lower) {
                return (*canonical).to_string();
            }
        }
    }

    // 4. Fallback: preserve custom or user-provided category as-is
    trimmed.to_string()
}

/// Whether `input` resolves to one of the standard FreeDesktop categories:
/// exact case-insensitive match, alias, or prefix match. `false` whenever
/// [`normalize_category`] would fall back to keeping the value as a custom
/// category (including empty input).
pub fn is_known_category(input: &str) -> bool {
    let normalized = normalize_category(input);
    CATEGORY_NAMES.contains(&normalized.as_str())
}

/// Best standard-category guess for `input`, or `None` when nothing is
/// reasonably close.
///
/// Scores standard names and aliases by small edit distance (≤ 2) first;
/// if none qualifies, falls back to containment (the shorter side must have
/// at least 4 characters). Callers only ask about unrecognized values; a
/// known value trivially matches itself with distance 0.
pub fn suggest_category(input: &str) -> Option<&'static str> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();

    let mut best: Option<(usize, &'static str)> = None;
    for cat in STANDARD_CATEGORIES {
        keep_close_candidate(
            &mut best,
            edit_distance(&lower, &cat.name.to_ascii_lowercase()),
            cat.name,
        );
    }
    for (alias, canonical) in CATEGORY_ALIASES {
        keep_close_candidate(&mut best, edit_distance(&lower, alias), canonical);
    }
    if let Some((_, candidate)) = best {
        return Some(candidate);
    }

    // Containment fallback: e.g. "videoeditor" contains "video",
    // "work" is contained in "network".
    let mut best_len = 0usize;
    let mut contained: Option<&'static str> = None;
    for cat in STANDARD_CATEGORIES {
        keep_contained_candidate(
            &lower,
            &cat.name.to_ascii_lowercase(),
            cat.name,
            &mut best_len,
            &mut contained,
        );
    }
    for (alias, canonical) in CATEGORY_ALIASES {
        keep_contained_candidate(&lower, alias, canonical, &mut best_len, &mut contained);
    }
    contained
}

/// Warning text for a value that [`normalize_category`] would keep as a
/// custom category, or `None` for known or empty values.
///
/// The value itself is still honored by the callers (warnings are advisory,
/// never fatal). `color` routes the text through [`crate::colors::yellow`].
pub fn unknown_category_warning(input: &str, color: bool) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || is_known_category(trimmed) {
        return None;
    }
    let headline = match suggest_category(trimmed) {
        Some(suggestion) => {
            format!("Unknown category '{trimmed}' — kept as custom. Did you mean: {suggestion}?")
        }
        None => format!("Unknown category '{trimmed}' — kept as custom."),
    };
    let hint =
        "  Run 'geico update <app> --categories' (sin valor) to see all standard categories.";
    Some(crate::colors::yellow(color, &format!("{headline}\n{hint}")))
}

/// Renders the read-only output of `--categories` used without a value.
///
/// * `current = Some((app, categories))` — update mode: prints the app's
///   current categories (`(none)` when empty), a blank line, the standard
///   categories table, and `tip`.
/// * `current = None` — add mode: just the table and `tip`.
pub fn render_categories_query(current: Option<(&str, &[String])>, tip: &str) -> String {
    let mut out = String::new();
    if let Some((app, categories)) = current {
        if categories.is_empty() {
            out.push_str(&format!("Current categories for '{app}': (none)\n"));
        } else {
            out.push_str(&format!(
                "Current categories for '{app}': {}\n",
                categories.join(",")
            ));
        }
        out.push('\n');
    }
    out.push_str(&format_categories_table());
    out.push_str(&format!("{DIM_GRAY}{tip}{RESET}\n"));
    out
}

/// Records `candidate` in `best` when it is within the accepted edit distance
/// and strictly closer than any candidate seen so far.
fn keep_close_candidate(
    best: &mut Option<(usize, &'static str)>,
    distance: usize,
    candidate: &'static str,
) {
    if distance > 2 {
        return;
    }
    match *best {
        Some((best_distance, _)) if distance >= best_distance => {}
        _ => *best = Some((distance, candidate)),
    }
}

/// Records `canonical` in `best` when `candidate` and `input` contain each
/// other (shorter side ≥ 4 chars); the longest match wins.
fn keep_contained_candidate(
    input: &str,
    candidate: &str,
    canonical: &'static str,
    best_len: &mut usize,
    best: &mut Option<&'static str>,
) {
    let hits = (candidate.len() >= 4 && input.contains(candidate))
        || (input.len() >= 4 && candidate.contains(input));
    if hits && candidate.len() > *best_len {
        *best_len = candidate.len();
        *best = Some(canonical);
    }
}

/// Levenshtein edit distance over characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Formats the standard categories into a human-readable table for terminal display.
pub fn format_categories_table() -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{BOLD_BLUE}Standard FreeDesktop Application Categories{RESET}\n\n"
    ));
    out.push_str(&format!(
        "{BOLD_CYAN}{:<14} {:<42} {}{RESET}\n",
        "CATEGORY", "DESCRIPTION", "EXAMPLES"
    ));
    out.push_str(&format!(
        "{DIM_GRAY}{:<14} {:<42} {}{RESET}\n",
        "--------------", "------------------------------------------", "-------------------------"
    ));

    for cat in STANDARD_CATEGORIES {
        out.push_str(&format!(
            "{BOLD_CYAN}{:<14}{RESET} {:<42} {DIM_GRAY}{}{RESET}\n",
            cat.name, cat.description, cat.examples
        ));
    }

    out.push_str(&format!(
        "\n{DIM_GRAY}Usage tip: pass multiple categories separated by commas, e.g.: --categories AudioVideo,Video{RESET}\n"
    ));
    out
}
