//! What the Bluetooth module can ask the outside world to do.

use api::JobId;

#[derive(Debug)]
pub enum ActionBluetooth {
    Sample,
    SetPowered {
        id: JobId,
        on: bool,
    },
    /// Timed discovery, so devices that only advertise while scanning show up.
    Scan,
    Connect {
        id: JobId,
        mac: String,
    },
    Disconnect {
        id: JobId,
        mac: String,
    },
    Notify {
        body: String,
    },
}
