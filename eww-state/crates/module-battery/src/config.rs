//! What this module has to be told rather than work out.

/// Twelve hours, which is what the panel's section label claims. Both come from
/// here so they cannot drift apart.
pub const SPAN: u64 = 12 * 60 * 60;

#[derive(Clone, Debug)]
pub struct ConfigBattery {
    /// Directory for the generated pill and chart.
    pub cache_dir: String,
    /// The chart's ring. Under the cache too, but named separately because it
    /// is the one file here that is worth keeping across a restart.
    pub history: String,
    pub poll: u32,
    /// Poll rate while the panel is open. The list is a ranking of live
    /// processes, so it has to move faster than the bar's pill does.
    pub menu_poll: u32,
    /// How long the process sampler watches for. Everything it reports is a
    /// difference between two reads this far apart, so it is the window the
    /// shares are shares *of*; too short and one scheduler slice decides the
    /// ranking, too long and the panel is stale by the time it draws.
    pub proc_window_ms: u32,
    /// How many rows the list shows.
    pub top: usize,
    pub span: u64,
    pub every: u64,
    /// Whether to draw for a dark menu bar. macOS's menu bar label is near-white
    /// in dark appearance and near-black in light, and these glyphs are
    /// generated rather than picked from a directory, so it has to be told.
    pub dark: bool,
    /// Shown against a process no dock entry claims.
    pub fallback_icon: String,
}

impl Default for ConfigBattery {
    fn default() -> Self {
        ConfigBattery {
            cache_dir: String::new(),
            history: String::new(),
            poll: 15_000,
            menu_poll: 5_000,
            proc_window_ms: 700,
            top: 5,
            span: SPAN,
            every: 60,
            dark: false,
            fallback_icon: String::new(),
        }
    }
}
