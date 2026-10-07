//! ANSI escape-code constants used for terminal output styling.

use std::io::IsTerminal;

/// Bold green — success messages.
pub const BOLD_GREEN: &str = "\x1b[1;32m";

/// Bold blue — headings / primary info.
pub const BOLD_BLUE: &str = "\x1b[1;34m";

/// Bold yellow — warnings.
pub const BOLD_YELLOW: &str = "\x1b[1;33m";

/// Bold red — errors.
pub const BOLD_RED: &str = "\x1b[1;31m";

/// Bold cyan — highlights / prompts.
pub const BOLD_CYAN: &str = "\x1b[1;36m";

/// Dim gray — secondary / de-emphasized text.
pub const DIM_GRAY: &str = "\x1b[90m";

/// Resets all attributes.
pub const RESET: &str = "\x1b[0m";

/// Whether ANSI colors should be emitted for output formatted right now.
///
/// Colors are disabled when `NO_COLOR` is set to any value, when
/// `CLICOLOR=0`, or when stdout is not a terminal.
pub fn should_color() -> bool {
    should_color_with(
        std::env::var_os("NO_COLOR").is_some(),
        std::env::var("CLICOLOR").ok().as_deref(),
        std::io::stdout().is_terminal(),
    )
}

/// Decision core of [`should_color`], split out so tests can cover
/// `NO_COLOR`, `CLICOLOR`, and TTY behavior without mutating process-wide
/// environment variables.
pub fn should_color_with(no_color: bool, clicolor: Option<&str>, stdout_is_tty: bool) -> bool {
    if no_color || clicolor == Some("0") {
        return false;
    }
    stdout_is_tty
}

/// Wraps `text` in `code` and [`RESET`] when `enabled`, else returns it unchanged.
fn paint(enabled: bool, code: &str, text: &str) -> String {
    if enabled {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Bold-green styling for success text, applied only when `enabled`.
pub fn green(enabled: bool, text: &str) -> String {
    paint(enabled, BOLD_GREEN, text)
}

/// Bold-yellow styling for warning text, applied only when `enabled`.
pub fn yellow(enabled: bool, text: &str) -> String {
    paint(enabled, BOLD_YELLOW, text)
}
