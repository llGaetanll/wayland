//! Everything that can move the machine forward.
//!
//! The machine has exactly one input: `Event`. Grouping the variants by where
//! they came from keeps a match readable as the program grows, and makes a
//! recorded log read as a story rather than a flat list.

use crate::event_clock::EventClock;
use crate::event_eww::EventEww;
use crate::event_job::EventJob;
use crate::event_probe::EventProbe;
use crate::event_ui::EventUi;

#[derive(Debug)]
pub enum Event {
    /// A click, a keybind, a CLI invocation.
    FromUi(EventUi),
    /// The clock moved.
    FromClock(EventClock),
    /// A probe is due, or one reported what it found.
    FromProbe(EventProbe),
    /// A side effect the machine dispatched has finished.
    FromJob(EventJob),
    /// The eww daemon said something about itself.
    FromEww(EventEww),
}
