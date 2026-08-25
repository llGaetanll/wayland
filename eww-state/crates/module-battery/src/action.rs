//! What the battery module can ask the outside world to do.

use api::JobId;

use crate::model::Battery;

#[derive(Debug)]
pub enum ActionBattery {
    Sample,
    /// Draw the pill for exactly this state.
    Render {
        id: JobId,
        battery: Battery,
    },
}
