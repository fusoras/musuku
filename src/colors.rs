//! ANSI escape-code constants used for terminal output styling.

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
