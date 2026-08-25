//! What the dock module can ask the outside world to do.

use crate::model::DockApp;

#[derive(Debug)]
pub enum ActionDock {
    Sample,
    /// Launch an app, detached from the daemon.
    Launch {
        cmd: String,
    },
    /// Write the new order to disk.
    Write {
        apps: Vec<DockApp>,
    },
}
