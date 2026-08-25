//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigBattery {
    /// Directory for the generated pill.
    pub cache_dir: String,
    pub poll: u32,
}

impl Default for ConfigBattery {
    fn default() -> Self {
        ConfigBattery {
            cache_dir: String::new(),
            poll: 15_000,
        }
    }
}
