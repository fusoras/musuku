use musu::colors::{dim, green, should_color_with, yellow};

#[test]
fn test_colors_should_color_disabled_when_no_color_env_is_set() {
    // NO_COLOR set with any value (including "0") must disable colors.
    assert!(!should_color_with(true, None, true));
    assert!(!should_color_with(true, Some("0"), true));
}

#[test]
fn test_colors_should_color_disabled_when_clicolor_is_zero() {
    assert!(!should_color_with(false, Some("0"), true));
}

#[test]
fn test_colors_should_color_disabled_when_stdout_is_not_a_tty() {
    assert!(!should_color_with(false, None, false));
}

#[test]
fn test_colors_should_color_enabled_on_tty() {
    assert!(should_color_with(false, None, true));
}

#[test]
fn test_colors_green_wraps_text_only_when_enabled() {
    assert_eq!(green(false, "[ok]"), "[ok]");
    assert_eq!(green(true, "[ok]"), "\x1b[1;32m[ok]\x1b[0m");
}

#[test]
fn test_colors_yellow_wraps_text_only_when_enabled() {
    assert_eq!(yellow(false, "[warning]"), "[warning]");
    assert_eq!(yellow(true, "[warning]"), "\x1b[1;33m[warning]\x1b[0m");
}

#[test]
fn test_colors_dim_wraps_text_only_when_enabled() {
    assert_eq!(dim(false, "[–]"), "[–]");
    assert_eq!(dim(true, "[–]"), "\x1b[90m[–]\x1b[0m");
}
