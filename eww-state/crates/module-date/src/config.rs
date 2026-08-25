//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigDate {
    pub poll: u32,
    /// A `date(1)` format string. The runtime owns what a calendar is.
    pub format: String,
}

impl Default for ConfigDate {
    fn default() -> Self {
        ConfigDate {
            poll: 20_000,
            format: "+%a %b %e  %H:%M".to_string(),
        }
    }
}
