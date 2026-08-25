//! Everything that can move the dock module forward.

use crate::model::DockApp;

#[derive(Debug)]
pub enum EventDock {
    /// A resync from disk is due.
    Due,
    Observed {
        apps: Vec<DockApp>,
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
