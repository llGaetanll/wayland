//! What the bar itself is told, as opposed to what a module is told.
//!
//! Two facts, and they are the ones no single widget can own: which dropdown
//! the user asked for, and which eww is on the other end of the wire.

use crate::menu::MenuId;
use crate::server_id::ServerId;

#[derive(Debug)]
pub enum EventBar {
    /// Open `menu` if closed, close it if already open, switch if another is up.
    MenuToggled {
        menu: MenuId,
    },
    MenuDismissed,
    /// What the last probe found at eww's socket: a server, or nothing there.
    /// The runtime reports; what it means is decided here.
    EwwObserved {
        server: Option<ServerId>,
    },
}
