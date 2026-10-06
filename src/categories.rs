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

/// Smartly normalizes a category string to official FreeDesktop standard name.
/// Handles exact case-insensitive matches, common aliases, and prefix matching.
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
    match lower.as_str() {
        "dev" | "programacion" | "programación" | "code" | "coding" | "ide" => {
            return "Development".to_string();
        }
        "vid" | "video" | "pelicula" | "peliculas" => return "Video".to_string(),
        "musica" | "música" | "music" | "sonido" | "audio" => return "Audio".to_string(),
        "multimedia" | "media" => return "AudioVideo".to_string(),
        "graficos" | "gráficos" | "grafico" | "gráfico" | "design" | "diseno" | "diseño"
        | "art" | "arte" | "3d" => return "Graphics".to_string(),
        "util" | "utils" | "utilidades" | "utilidad" | "tools" | "tool" | "herramienta"
        | "herramientas" => return "Utility".to_string(),
        "web" | "internet" | "browser" | "navegador" | "red" | "redes" => {
            return "Network".to_string();
        }
        "juegos" | "juego" | "games" | "game" => return "Game".to_string(),
        "ofimatica" | "ofimática" | "documentos" | "docs" => return "Office".to_string(),
        "ciencia" | "ciencias" | "science" | "math" | "matematicas" | "matemáticas" => {
            return "Science".to_string();
        }
        "config" | "configuracion" | "configuración" | "ajustes" | "preferencias" => {
            return "Settings".to_string();
        }
        "sistema" | "sys" => return "System".to_string(),
        "educacion" | "educación" | "educativo" | "learn" | "learning" => {
            return "Education".to_string();
        }
        _ => {}
    }

    // 3. Prefix matching against standard names (min length 3)
    if lower.len() >= 3 {
        for cat in STANDARD_CATEGORIES {
            if cat.name.to_ascii_lowercase().starts_with(&lower) {
                return cat.name.to_string();
            }
        }
    }

    // 4. Fallback: preserve custom or user-provided category as-is
    trimmed.to_string()
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
