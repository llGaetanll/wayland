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
    /// Kept a whole number of rows, so a long list never ends on a row sliced
    /// in half by the bottom of the panel.
    pub max_list_height: u32,
}

impl Default for ConfigBluetooth {
    fn default() -> Self {
        ConfigBluetooth {
            icons_dir: String::new(),
            poll_idle: 6_000,
            poll_open: 5_000,
            job_timeout: Millis::secs(30),
            row_height: 26,
            max_list_height: 280,
        }
    }
}

impl ConfigBluetooth {
    /// Point at a different icon set. The glyphs are one colour baked into the
    /// file, so light and dark are two directories rather than one recoloured
    /// at draw time — and because eww and GTK cache by path, moving between
    /// them is what makes the icon actually reload.
    pub fn set_icons_dir(&mut self, dir: &str) {
        self.icons_dir = dir.to_string();
    }

    pub(crate) fn icon(&self, name: &str) -> String {
        format!("{}/{}.svg", self.icons_dir, name)
    }

    /// The glyph for one device row. A connected device sits in a filled accent
    /// circle, so its glyph has to be the white one to read against it.
    pub(crate) fn device_icon(&self, connected: bool) -> String {
        self.icon(if connected {
            "bluetooth-on-symbolic"
        } else {
            "bluetooth-active-symbolic"
        })
    }

    pub(crate) fn list_height(&self, n: usize) -> i64 {
        let h = (n as u32).saturating_mul(self.row_height);
        h.clamp(self.row_height, self.max_list_height) as i64
    }
}
