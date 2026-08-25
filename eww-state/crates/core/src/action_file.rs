//! Touching a file we own.

use crate::model::DockApp;

#[derive(Debug)]
pub enum ActionFile {
    /// Write the new dock order to disk.
    WriteDock {
        apps: Vec<DockApp>,
    },
    /// Regenerate the battery pill SVG for this exact state.
    RenderBattery {
        percent: u8,
        charging: bool,
    },
    /// Move a staged capture into the screenshots folder.
    SaveShot {
        path: String,
    },
    DeleteShot {
        path: String,
    },
}
