//! Everything that can move the volume module forward.

use crate::model::Audio;
use crate::model::AudioOutputs;

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
    /// The Sound dropdown came into or went out of view. The outputs are only
    /// worth listing while it is up: nothing in the bar shows them, and reading
    /// them costs a full `pactl list sinks` where the volume costs one line.
    MenuOpened,
    MenuClosed,
    /// An output row was clicked.
    OutputPicked {
        id: String,
    },
    OutputsObserved {
        outputs: AudioOutputs,
    },
    /// A switch reported back. `ok` false leaves the list alone and lets the
    /// next sample say where sound is actually going.
    OutputSwitched {
        ok: bool,
    },
}
