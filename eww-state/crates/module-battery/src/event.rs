//! Everything that can move the battery module forward.

use api::JobId;

use crate::model::Battery;

#[derive(Debug)]
pub enum EventBattery {
    /// A sample is due, from a timer or because someone asked.
    Due,
    Observed {
        battery: Battery,
    },
    /// A render finished. `ok` decides whether the bar may point at the file.
    Finished {
        id: JobId,
        ok: bool,
    },
}
