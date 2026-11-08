//! Everything that can move the screenshot module forward.

use api::JobId;
use api::Millis;

use crate::model::Shot;
use crate::model::ShotAction;

#[derive(Debug)]
pub enum EventShot {
    /// Print was pressed.
    Requested,
    /// A capture finished and left this file behind.
    Staged {
        shot: Shot,
    },
    /// A button on the preview was clicked.
    Acted {
        action: ShotAction,
    },
    /// The clock moved; give up on a selection nobody is making any more.
    Tick {
        now: Millis,
    },
    Finished {
        id: JobId,
    },
}
