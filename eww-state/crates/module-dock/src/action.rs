//! What the dock module can ask the outside world to do.

use crate::model::DockApp;

#[derive(Debug)]
pub enum ActionDock {
    /// Re-read `dock.conf`.
    Sample,
    /// Ask the compositor which of these ids has a window open. The ids ride
    /// along because performing an action is pure IO: it is handed everything
    /// it needs and never reaches back into the state that asked.
    SampleRunning { ids: Vec<String> },
    /// Launch an app, detached from the daemon.
    Launch { cmd: String },
    /// Write the new order to disk.
    Write { apps: Vec<DockApp> },
}
