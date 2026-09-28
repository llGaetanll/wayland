//! What this module has to be told rather than work out.

#[derive(Clone, Debug)]
pub struct ConfigSearch {
    /// How many results the panel shows at once.
    pub max_results: usize,
    /// Row height in px, for sizing the results list. Matches `.search-row` in
    /// eww.scss — CSS cannot report its own height, so the two are kept in
    /// step by hand the way the Wi-Fi list is.
    pub row_height: u32,
    /// How a `Terminal=true` entry is launched: this, then the command.
    pub terminal: String,
}

impl Default for ConfigSearch {
    fn default() -> Self {
        ConfigSearch {
            max_results: 8,
            row_height: 44,
            terminal: "alacritty -e".to_string(),
        }
    }
}

impl ConfigSearch {
    /// Height of the results list, in px. Zero when there is nothing to show,
    /// which is what collapses the panel back to a bare field.
    pub(crate) fn list_height(&self, n: usize) -> i64 {
        (n.min(self.max_results) as u32).saturating_mul(self.row_height) as i64
    }
}
