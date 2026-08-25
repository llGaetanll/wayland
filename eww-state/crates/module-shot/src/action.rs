//! What the screenshot module can ask the outside world to do.

use api::JobId;

#[derive(Debug)]
pub enum ActionShot {
    /// Run the region selector and stage the result. Long-running and
    /// interactive; the module only cares that exactly one is ever in flight,
    /// which is what the id is for.
    Capture {
        id: JobId,
    },
    Copy {
        path: String,
    },
    Save {
        path: String,
    },
    Delete {
        path: String,
    },
}
