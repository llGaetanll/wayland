//! Running a program.
//!
//! Everything here shells out. What the program is called and how it is
//! invoked is the runtime's business; the machine only says what it wants.

#[derive(Debug)]
pub enum ActionShell {
    SetVolume {
        percent: u8,
    },
    SetBrightness {
        percent: u8,
    },
    WifiSetEnabled {
        on: bool,
    },
    WifiScan,
    /// Join the network, prompting for a passphrase if it is unknown and secured.
    WifiConnect {
        ssid: String,
    },
    BtSetPowered {
        on: bool,
    },
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
    /// Run the region selector and stage the result. Long-running and
    /// interactive; the machine only cares that exactly one is ever in flight.
    Capture,
    CopyShot {
        path: String,
    },
    Notify {
        title: String,
        body: String,
    },
}
