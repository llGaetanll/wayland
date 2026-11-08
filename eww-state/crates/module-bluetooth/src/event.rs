//! Everything that can move the Bluetooth module forward.

use api::JobId;
use api::Millis;

use crate::model::BtSnapshot;

#[derive(Debug)]
pub enum EventBluetooth {
    /// A sample is due, from a timer or because someone asked.
    Due,
    /// The switch in the dropdown was clicked.
    Toggled,
    /// A device row was clicked: connect it, or disconnect it if it is already
    /// connected.
    Picked {
        mac: String,
    },
    Observed {
        bt: BtSnapshot,
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
    },
}
