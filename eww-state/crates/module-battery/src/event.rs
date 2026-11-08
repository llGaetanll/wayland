//! Everything that can move the battery module forward.

use api::JobId;

use crate::model::HistPoint;
use crate::model::Proc;
use crate::model::Reading;

#[derive(Debug)]
pub enum EventBattery {
    /// A sample is due, from a timer or because someone asked.
    Due,
    Observed {
        reading: Reading,
        /// The chart's ring, as the probe found it after adding this sample.
        /// It comes back with the reading rather than being accumulated here:
        /// the ring lives on disk, so the file is the truth and a copy kept
        /// beside it would be a second one.
        history: Vec<HistPoint>,
    },
    /// The panel came into or out of view. Both halves of the panel are
    /// expensive — the chart is a render, the list is a timed sample of every
    /// process on the machine — and neither is worth anything off screen.
    MenuOpened,
    MenuClosed,
    ProcsObserved {
        procs: Vec<Proc>,
    },
    /// A pill render finished. `ok` decides whether the bar may point at it.
    Finished {
        id: JobId,
        ok: bool,
    },
    /// A chart render finished, with the same rule.
    GraphFinished {
        id: JobId,
        ok: bool,
    },
}
