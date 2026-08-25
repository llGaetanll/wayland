//! What the bar itself is told, as opposed to what a module is told.
//!
//! These are the facts no single widget owns: which dropdown the user asked
//! for, and whether a fullscreen Space is focused.

use crate::menu::MenuId;

#[derive(Debug)]
pub enum EventBar {
    /// Open `menu` if closed, close it if already open, switch if another is up.
    MenuToggled {
        menu: MenuId,
    },
    MenuDismissed,
    /// A fullscreen re-check is due, because the compositor said something that
    /// could have changed the answer.
    FullscreenDue,
    /// The focused workspace holds a fullscreen window, so bar and dock hide.
    Fullscreen {
        on: bool,
    },
    /// eww came back after a restart. Nothing changes here: the runtime
    /// republishes from the state we already hold, which is what recovery
    /// needs.
    EwwRestarted,
}
