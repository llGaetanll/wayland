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
        /// Performing an action is pure IO: it is handed everything it needs
        /// and never reaches back into the state that asked.
        dark: bool,
    },
}
