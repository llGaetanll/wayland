//! What the Wi-Fi module can ask the outside world to do.
//!
//! Only the two the module waits on carry a [`JobId`]: a rescan and a
//! notification are fire-and-forget, and giving them a name nobody says would
//! be a name nobody says.

use api::JobId;

#[derive(Debug)]
pub enum ActionWifi {
    /// Sample the world now, whatever the interval says. The result arrives as
    /// an observation rather than as a completion, so it needs no id.
    Sample,
    SetEnabled {
        id: JobId,
        on: bool,
    },
    Scan,
    /// Join the network, prompting for a passphrase if it is unknown and secured.
    Connect {
        id: JobId,
        ssid: String,
    },
    Notify {
        body: String,
    },
}
