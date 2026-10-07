//! Terminal loading spinner for background operations.

use std::io::{IsTerminal, Write, stderr};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Spinner {
    running: Arc<AtomicBool>,
    rendered: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Starts a spinner on `stderr` with a 200ms grace period if it is an interactive terminal.
    pub fn start(message: &'static str) -> Self {
        Self::start_with_delay(message, Duration::from_millis(200), stderr().is_terminal())
    }

    /// Starts a spinner unconditionally without delay (useful for testing).
    pub fn start_forced(message: &'static str) -> Self {
        Self::start_with_delay(message, Duration::ZERO, true)
    }

    /// Starts a spinner with a custom grace period delay before emitting output.
    pub fn start_with_delay(message: &'static str, delay: Duration, is_terminal: bool) -> Self {
        if !is_terminal {
            return Self {
                running: Arc::new(AtomicBool::new(false)),
                rendered: Arc::new(AtomicBool::new(false)),
                handle: None,
            };
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let rendered = Arc::new(AtomicBool::new(false));
        let rendered_clone = rendered.clone();

        let handle = thread::spawn(move || {
            if delay > Duration::ZERO {
                thread::sleep(delay);
                if !running_clone.load(Ordering::Relaxed) {
                    return;
                }
            }

            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let mut i = 0;
            let mut err = stderr();
            let colored = crate::colors::should_color();
            let base_message = message.trim_end_matches('.');

            while running_clone.load(Ordering::Relaxed) {
                rendered_clone.store(true, Ordering::Relaxed);
                let frame = frames[i % frames.len()];
                let dots = animated_dots(i);
                if colored {
                    let _ = write!(
                        err,
                        "\r\x1b[2K\x1b[36m{}\x1b[0m {}{}",
                        frame, base_message, dots
                    );
                } else {
                    let _ = write!(err, "\r\x1b[2K{} {}{}", frame, base_message, dots);
                }
                let _ = err.flush();
                i += 1;
                thread::sleep(Duration::from_millis(80));
            }

            if rendered_clone.load(Ordering::Relaxed) {
                let _ = write!(err, "\r\x1b[2K\r");
                let _ = err.flush();
            }
        });

        Self {
            running,
            rendered,
            handle: Some(handle),
        }
    }

    /// Returns true if the spinner thread is currently running.
    pub fn is_active(&self) -> bool {
        self.running.load(Ordering::Relaxed) && self.handle.is_some()
    }

    /// Returns true if the spinner has already rendered at least one frame.
    pub fn has_rendered(&self) -> bool {
        self.rendered.load(Ordering::Relaxed)
    }

    /// Stops the spinner and erases the line from the terminal if rendered.
    pub fn stop(mut self) {
        self.stop_internal();
    }

    fn stop_internal(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.running.store(false, Ordering::Relaxed);
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop_internal();
    }
}

pub fn animated_dots(tick: usize) -> &'static str {
    match (tick / 4) % 4 {
        0 => "   ",
        1 => ".  ",
        2 => ".. ",
        _ => "...",
    }
}
