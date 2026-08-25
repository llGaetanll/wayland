//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigTheme {
    pub poll: u32,
}

impl Default for ConfigTheme {
    fn default() -> Self {
        ConfigTheme { poll: 5_000 }
    }
}
