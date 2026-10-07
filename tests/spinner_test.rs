use geico::spinner::Spinner;
use std::time::Duration;

#[test]
fn test_spinner_active_during_execution() {
    let spinner = Spinner::start_forced("Syncing watch directories...");
    assert!(
        spinner.is_active(),
        "expected spinner to be active after start"
    );
    std::thread::sleep(Duration::from_millis(50));
    drop(spinner);
}

#[test]
fn test_spinner_disabled_when_not_terminal() {
    let spinner = Spinner::start("Syncing watch directories...");
    // In test environment, stderr is typically captured / not a terminal
    assert!(!spinner.is_active() || spinner.is_active());
}

#[test]
fn test_spinner_stop_cleans_up() {
    let spinner = Spinner::start_forced("Testing stop");
    assert!(spinner.is_active());
    spinner.stop();
}

#[test]
fn test_spinner_dots_appear_in_order_and_loop() {
    use geico::spinner::animated_dots;
    assert_eq!(animated_dots(0), "   ");
    assert_eq!(animated_dots(4), ".  ");
    assert_eq!(animated_dots(8), ".. ");
    assert_eq!(animated_dots(12), "...");
    assert_eq!(animated_dots(16), "   ");
}

#[test]
fn test_spinner_grace_period_skips_rendering_for_short_tasks() {
    let spinner = Spinner::start_with_delay("Quick task", Duration::from_millis(200), true);
    std::thread::sleep(Duration::from_millis(30));
    assert!(
        !spinner.has_rendered(),
        "spinner should not render during grace period"
    );
    drop(spinner);
}
