//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigBattery {
    /// Directory for the generated pill.
    pub cache_dir: String,
    pub poll: u32,
    /// Whether to draw the glyph for a dark menu bar. macOS's menu bar label is
    /// near-white in dark appearance and near-black in light, and this glyph is
    /// generated rather than picked from a directory, so it has to be told.
    pub dark: bool,
}

impl Default for ConfigBattery {
    fn default() -> Self {
        ConfigBattery {
            cache_dir: String::new(),
            poll: 15_000,
            dark: false,
        }
    }
}
