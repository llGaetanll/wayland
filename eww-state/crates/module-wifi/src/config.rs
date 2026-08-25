//! What this module has to be told rather than work out.

use api::millis::Millis;

#[derive(Clone, Debug)]
pub struct ConfigWifi {
    /// Directory holding the symbolic status icons (`.svg` files).
    pub icons_dir: String,
    /// Sampled this often with the dropdown shut, and this often with it open:
    /// the list wants to be fresher than the bar icon does.
    pub poll_idle: u32,
    pub poll_open: u32,
    /// How long a command may stay in flight before the module stops believing
    /// in it. Without this a result that never comes back — a killed daemon, a
    /// hung `nmcli` — leaves the switch stuck mid-flip forever.
    pub job_timeout: Millis,
    /// Rows taller than this collapse the menu scroll area; matches the CSS.
    pub row_height: u32,
    pub max_list_height: u32,
}

impl Default for ConfigWifi {
    fn default() -> Self {
        ConfigWifi {
            icons_dir: String::new(),
            poll_idle: 6_000,
            poll_open: 5_000,
            job_timeout: Millis::secs(30),
            row_height: 40,
            max_list_height: 300,
        }
    }
}

impl ConfigWifi {
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
