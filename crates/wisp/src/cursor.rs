use std::time::Instant;

/// Blink interval in milliseconds (GPUI: 500ms).
const BLINK_INTERVAL: u64 = 500;
/// Pause delay in milliseconds — cursor stays solid after input (GPUI: 300ms).
const PAUSE_DELAY: u64 = 300;

/// Manages cursor blink state, matching GPUI's `BlinkCursor` behavior.
///
/// - Blinks every 500ms
/// - Stays solid for 300ms after `mark_activity()` is called
/// - Starts visible (like GPUI's paused state on focus)
/// - Can be told to stop blinking when window is inactive
#[derive(Debug, Clone)]
pub struct CursorBlink {
    /// When the last user activity (key press) occurred.
    last_activity: Instant,
    /// When the current blink epoch started.
    epoch_start: Instant,
    /// Whether the window is active (defaults to true).
    window_active: bool,
}

impl CursorBlink {
    pub fn new() -> Self {
        Self {
            last_activity: Instant::now(),
            epoch_start: Instant::now(),
            window_active: true,
        }
    }

    /// Call on any user input (key press, mouse click, etc.).
    /// Keeps cursor solid for `PAUSE_DELAY` ms, then resumes blinking.
    pub fn mark_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Set whether the window is active (has keyboard focus).
    /// When inactive, cursor is hidden (matches GPUI `window.is_window_active()`).
    pub fn set_window_active(&mut self, active: bool) {
        self.window_active = active;
        if active {
            self.epoch_start = Instant::now();
        }
    }

    /// Returns true if the cursor should be visible right now.
    pub fn visible(&self) -> bool {
        if !self.window_active {
            return false;
        }

        // Pause period: solid cursor after activity
        let elapsed = self.last_activity.elapsed().as_millis();
        if elapsed < PAUSE_DELAY as u128 {
            return true;
        }

        // Normal blink: visible on even periods
        let blink_elapsed = self.epoch_start.elapsed().as_millis();
        (blink_elapsed / BLINK_INTERVAL as u128).is_multiple_of(2)
    }

    /// For imperative use: mark_activity then check visible in one call.
    pub fn mark_and_visible(&mut self) -> bool {
        self.mark_activity();
        true
    }
}

impl Default for CursorBlink {
    fn default() -> Self {
        Self::new()
    }
}
