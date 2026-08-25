//! Everything that can move the volume module forward.

use crate::model::Audio;

#[derive(Debug)]
pub enum EventAudio {
    /// A sample is due, from a timer or because someone asked.
    Due,
    /// The user moved the slider.
    Set {
        percent: u8,
    },
    Observed {
        audio: Audio,
    },
    Finished,
}
