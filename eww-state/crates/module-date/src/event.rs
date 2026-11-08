//! Everything that can move the date module forward.

#[derive(Debug)]
pub enum EventDate {
    /// A sample is due, from a timer or because someone asked.
    Due,
    Observed {
        text: String,
    },
}
