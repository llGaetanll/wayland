//! Everything that can move the dock module forward.

use crate::model::DockApp;

#[derive(Debug)]
pub enum EventDock {
    /// A tick is due. What that costs is the module's business: usually just
    /// asking the compositor who is running, and only rarely re-reading disk.
    Due,
    Observed {
        apps: Vec<DockApp>,
    },
    /// The ids of the apps the compositor currently has a window for.
    Running {
        ids: Vec<String>,
    },
    /// An icon was clicked.
    Launched {
        id: String,
    },
    /// One app was dropped onto another; they trade places.
    Dropped {
        moved: String,
        onto: String,
    },
}
