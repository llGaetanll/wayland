//! What this module has to be told rather than work out.

use api::Millis;

#[derive(Clone, Debug)]
pub struct ConfigBluetooth {
    /// Directory holding the symbolic status icons (`.svg` files).
    pub icons_dir: String,
    pub poll_idle: u32,
    pub poll_open: u32,
    /// How long a command may stay in flight before the module stops believing
    /// in it. `bluetoothctl connect` on a device that has wandered out of range
    /// simply never returns.
    pub job_timeout: Millis,
    /// Rows taller than this collapse the menu scroll area; matches the CSS.
    pub row_height: u32,
    pub max_list_height: u32,
}

impl Default for ConfigBluetooth {
    fn default() -> Self {
        ConfigBluetooth {
            icons_dir: String::new(),
            poll_idle: 6_000,
            poll_open: 5_000,
            job_timeout: Millis::secs(30),
            row_height: 40,
            max_list_height: 300,
        }
    }
}

impl ConfigBluetooth {
    pub(crate) fn icon(&self, name: &str) -> String {
        format!("{}/{}.svg", self.icons_dir, name)
    }

    pub(crate) fn list_height(&self, n: usize) -> i64 {
        let h = (n as u32).saturating_mul(self.row_height);
        h.clamp(self.row_height, self.max_list_height) as i64
    }
}
