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
    /// Rows taller than this collapse the menu scroll area; matches the CSS
    /// (`.row` in eww.scss), which is 26 to match the macOS panel capture.
    pub row_height: u32,
    /// How many networks to put in the panel at all.
    ///
    /// Not cosmetic. A scan here routinely returns 50 access points, and
    /// building 50 rows — each a button holding a background-image glyph — was
    /// enough that the panel's layer surface intermittently failed to map: the
    /// menu opened, eww listed the window as active, and nothing was ever
    /// drawn. It is also not what the thing we are copying does: macOS lists
    /// the network in use under "Known Network" and hides the rest behind an
    /// "Other Networks" submenu. Sorted by signal, so the cap drops the
    /// unreachable tail first.
    pub max_rows: usize,
    /// Kept a whole number of rows, so a long list never ends on a row sliced
    /// in half by the bottom of the panel.
    pub max_list_height: u32,
}

impl Default for ConfigWifi {
    fn default() -> Self {
        ConfigWifi {
            icons_dir: String::new(),
            poll_idle: 6_000,
            poll_open: 5_000,
            job_timeout: Millis::secs(30),
            row_height: 26,
            max_list_height: 260,
            max_rows: 16,
        }
    }
}

impl ConfigWifi {
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

    /// Scroll-area height for a list of `n` rows, clamped so the panel neither
    /// collapses to nothing nor runs off the screen.
    pub(crate) fn list_height(&self, n: usize) -> i64 {
        let h = (n as u32).saturating_mul(self.row_height);
        h.clamp(self.row_height, self.max_list_height) as i64
    }
}
