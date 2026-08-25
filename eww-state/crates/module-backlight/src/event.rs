//! Everything that can move the brightness module forward.

#[derive(Debug)]
pub enum EventBacklight {
    /// A sample is due, from a timer or because someone asked.
    Due,
    Set {
        percent: u8,
    },
    Observed {
        percent: u8,
    },
    Finished,
}
