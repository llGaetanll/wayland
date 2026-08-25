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
    pub(crate) fn icon(&self, name: &str) -> String {
        format!("{}/{}.svg", self.icons_dir, name)
    }
}
