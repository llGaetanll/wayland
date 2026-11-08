//! Everything that can move the Wi-Fi module forward.
//!
//! The variant names say where each one came from: a click, a sample, a job
//! reporting back, the clock. The parent does the translating, so nothing here
//! has to know that a bar exists.

use api::jobs::JobId;
use api::millis::Millis;

use crate::model::NetSnapshot;

#[derive(Debug)]
pub enum EventWifi {
    /// A sample is due, from a timer or because someone asked.
    Due,
    /// The switch in the dropdown was clicked.
    Toggled,
    /// A network row was clicked.
    Picked {
        ssid: String,
    },
    /// A sample arrived.
    Observed {
        net: NetSnapshot,
    },
    MenuOpened,
    MenuClosed,
    /// The clock moved; give up on anything that has waited too long.
    Tick {
        now: Millis,
    },
    Finished {
        id: JobId,
        ok: bool,
        /// Why it failed, when there is a rule keyed on the reason.
        detail: Option<String>,
    },
}
