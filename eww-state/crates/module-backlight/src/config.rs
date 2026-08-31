//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigBacklight {
    /// Directory holding the symbolic status icons (`.svg` files).
    pub icons_dir: String,
    pub poll: u32,
}

impl Default for ConfigBacklight {
    fn default() -> Self {
        ConfigBacklight {
            icons_dir: String::new(),
            poll: 3_000,
        }
    }
}

impl ConfigBacklight {
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
}
