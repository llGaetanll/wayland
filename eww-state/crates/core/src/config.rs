//! Static configuration handed to the state machine at construction.
//!
//! Everything the machine would otherwise have to learn from the environment —
//! where icons live, how often to poll, how long to wait — arrives here as
//! plain data, so the machine itself stays free of IO.

use crate::millis::Millis;

#[derive(Clone, Debug)]
pub struct Config {
    /// Directory holding the symbolic status icons (`.svg` files).
    pub icons_dir: String,
    /// Directory for generated assets (the battery pill SVG).
    pub cache_dir: String,
    pub poll: PollConfig,
    /// How long a dispatched job may stay in flight before the machine stops
    /// believing in it. Without this a result that never comes back — a killed
    /// daemon, a hung `bluetoothctl` — leaves a switch stuck mid-flip forever.
    pub job_timeout: Millis,
    /// The same, for a region selection: the user is holding the mouse, so this
    /// is patience rather than a stuck job.
    pub capture_timeout: Millis,
    /// Rows taller than this collapse the menu scroll area; matches the CSS.
    pub row_height: u32,
    pub max_list_height: u32,
    /// Widest the screenshot preview may be drawn.
    pub shot_max_width: u32,
}

/// Probe intervals, in milliseconds. A probe whose interval is 0 never runs on
/// a timer and only fires when the machine explicitly asks for it.
#[derive(Clone, Copy, Debug)]
pub struct PollConfig {
    pub date: u32,
    pub battery: u32,
    pub theme: u32,
    pub audio: u32,
    pub backlight: u32,
    /// Network and Bluetooth are sampled in full on every tick; the open menu
    /// just wants them fresher than the bar icon does.
    pub net_idle: u32,
    pub net_open: u32,
    pub bt_idle: u32,
    pub bt_open: u32,
    /// The dock order only changes when we change it, so this is a lazy resync.
    pub dock: u32,
}

impl Default for PollConfig {
    fn default() -> Self {
        PollConfig {
            date: 20_000,
            battery: 15_000,
            theme: 5_000,
            audio: 2_000,
            backlight: 3_000,
            net_idle: 6_000,
            net_open: 5_000,
            bt_idle: 6_000,
            bt_open: 5_000,
            dock: 3_600_000,
        }
    }
}

impl Config {
    pub fn new(icons_dir: impl Into<String>, cache_dir: impl Into<String>) -> Self {
        Config {
            icons_dir: icons_dir.into(),
            cache_dir: cache_dir.into(),
            poll: PollConfig::default(),
            job_timeout: Millis::secs(30),
            capture_timeout: Millis::secs(180),
            row_height: 40,
            max_list_height: 300,
            shot_max_width: 320,
        }
    }

    pub(crate) fn icon(&self, name: &str) -> String {
        format!("{}/{}.svg", self.icons_dir, name)
    }

    /// Scroll-area height for a list of `n` rows, clamped so the panel neither
    /// collapses to nothing nor runs off the screen.
    pub(crate) fn list_height(&self, n: usize) -> i64 {
        let h = (n as u32).saturating_mul(self.row_height);
        h.clamp(self.row_height, self.max_list_height) as i64
    }
}
