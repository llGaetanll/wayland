//! Side effects the machine asks the shell to perform.
//!
//! An `Action` is a request, not a result: nothing here runs, and the machine
//! never waits on one. When the outcome matters, the shell reports it back as
//! `Event::JobDone` carrying the same `JobId`.

use crate::event::{JobId, Probe};
use crate::model::DockApp;

#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub id: JobId,
    pub kind: ActionKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ActionKind {
    /// Sample the world now.
    Probe(Probe),
    SetVolume(u8),
    SetBrightness(u8),
    WifiSetEnabled(bool),
    WifiScan,
    /// Join the network, prompting for a passphrase if it is unknown and secured.
    WifiConnect {
        ssid: String,
    },
    BtSetPowered(bool),
    BtScan,
    BtConnect {
        mac: String,
        name: String,
    },
    BtDisconnect {
        mac: String,
        name: String,
    },
    /// Launch a dock app, detached from the daemon.
    Launch {
        cmd: String,
    },
    /// Write the new dock order to disk.
    PersistDock(Vec<DockApp>),
    /// Regenerate the battery pill SVG for this exact state.
    RenderBattery {
        percent: u8,
        charging: bool,
    },
    /// Run the region selector and stage the result. Long-running and
    /// interactive; the widget only cares that exactly one is ever in flight.
    ShotCapture,
    ShotCopy {
        path: String,
    },
    ShotSave {
        path: String,
    },
    ShotDelete {
        path: String,
    },
    Notify {
        title: String,
        body: String,
    },
}
