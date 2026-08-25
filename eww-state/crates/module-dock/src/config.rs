//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigDock {
    /// The order only changes when we change it, so this is a lazy resync.
    pub poll: u32,
}

impl Default for ConfigDock {
    fn default() -> Self {
        ConfigDock { poll: 3_600_000 }
    }
}
