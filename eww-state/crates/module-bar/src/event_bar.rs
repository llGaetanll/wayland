//! What the bar itself is told, as opposed to what a module is told.
//!
//! Two facts, and they are the ones no single widget can own: which dropdown
//! the user asked for, and which eww is on the other end of the wire — which
//! means both who is listening and whether they still hold what we published.

use crate::eww_gen::EwwGen;
use crate::menu::MenuId;
use crate::server_id::ServerId;

#[derive(Debug)]
pub enum EventBar {
    /// Open `menu` if closed, close it if already open, switch if another is up.
    MenuToggled {
        menu: MenuId,
    },
    MenuDismissed,
    /// What the last probe found at eww's socket: a server, or nothing there,
    /// and the generation stamped in that server's variable table. `None` for
    /// the generation is a table nobody has stamped — a server that has just
    /// started, or one that has just re-read its config and thrown every
    /// variable away. The runtime reports; what it means is decided here.
    EwwObserved {
        server: Option<ServerId>,
        gen: Option<EwwGen>,
    },
}
