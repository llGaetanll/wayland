//! What a dispatched side effect had to say for itself.

use crate::model::Shot;
use crate::probe::JobId;

#[derive(Debug)]
pub enum EventJob {
    Finished {
        id: JobId,
        ok: bool,
        /// Why it failed, when the machine has a rule keyed on the reason.
        detail: Option<String>,
    },
    /// A capture finished and left this file behind.
    ShotStaged { shot: Shot },
}
