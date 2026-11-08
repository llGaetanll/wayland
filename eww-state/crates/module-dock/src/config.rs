//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigDock {
    /// The order only changes when we change it, so this is a lazy resync.
    pub poll: u32,
    /// How often to ask the compositor which apps have a window open. This is
    /// the rate the module actually ticks at; the disk resync above rides on a
    /// count of these, because two timers for one module would be two things to
    /// keep in step.
    pub running_poll: u32,
    /// What clicking the Trash runs. The Trash is a fixture, not a dock app —
    /// it cannot be reordered, dropped onto, or removed — so it is named here
    /// rather than in `dock.conf`.
    pub trash_cmd: String,
}

impl Default for ConfigDock {
    fn default() -> Self {
        ConfigDock {
            poll: 3_600_000,
            running_poll: 2_000,
            trash_cmd: "xdg-open trash:///".to_string(),
        }
    }
}
