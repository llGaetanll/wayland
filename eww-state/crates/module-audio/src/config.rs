//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigAudio {
    /// Directory holding the symbolic status icons (`.svg` files).
    pub icons_dir: String,
    pub poll: u32,
}

impl Default for ConfigAudio {
    fn default() -> Self {
        ConfigAudio {
            icons_dir: String::new(),
            poll: 2_000,
        }
    }
}

impl ConfigAudio {
    pub(crate) fn icon(&self, name: &str) -> String {
        format!("{}/{}.svg", self.icons_dir, name)
    }
}
