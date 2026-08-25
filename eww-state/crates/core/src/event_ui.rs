//! What the user asked for.
//!
//! Every one of these arrives through the control socket: a widget handler, a
//! compositor keybind, or someone typing `eww-state` at a prompt.

use crate::model::MenuId;
use crate::model::ShotAction;

#[derive(Debug)]
pub enum EventUi {
    /// Open `menu` if closed, close it if already open, switch if another is up.
    MenuToggled {
        menu: MenuId,
    },
    MenuDismissed,
    VolumeSet {
        percent: u8,
    },
    BrightnessSet {
        percent: u8,
    },
    WifiToggled,
    WifiPicked {
        ssid: String,
    },
    BtToggled,
    /// Connect the device, or disconnect it when it is already connected.
    BtPicked {
        mac: String,
    },
    DockLaunched {
        id: String,
    },
    /// The user dropped one dock app onto another; they trade places.
    DockDropped {
        moved: String,
        onto: String,
    },
    /// Take a screenshot: region select, then stage it for preview.
    ShotRequested,
    ShotActed {
        action: ShotAction,
    },
}
