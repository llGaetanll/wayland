//! Everything that can move the theme module forward.

use crate::model::Theme;

#[derive(Debug)]
pub enum EventTheme {
    /// A sample is due, from a timer or because someone asked.
    Due,
    Observed {
        theme: Theme,
    },
}
