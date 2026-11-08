//! Everything that can move the Spaces module forward.

use api::JobId;

use crate::model::Snapshot;
use crate::model::WindowAddr;

#[derive(Debug)]
pub enum EventSpaces {
    /// The compositor said something that could have changed the layout.
    Due,
    /// What one look at the compositor found.
    ///
    /// The id says *which* look, which is the whole of how a move of our own is
    /// told apart from the user's doing — see [`StateSpaces`](crate::state).
    Sampled {
        id: JobId,
        snapshot: Snapshot,
    },
    /// The user asked to close the focused window.
    CloseRequested,
    /// The user asked to minimize the focused window.
    MinimizeRequested,
    /// A minimized window's dock tile was clicked. Named, unlike the two above,
    /// because the window being asked for is not the focused one — by
    /// definition, it is not on screen at all.
    RestoreRequested {
        addr: WindowAddr,
    },
    /// A window's photograph is on disk.
    ///
    /// Separate from the move finishing, and the whole reason the thumbnail is
    /// not published the moment a window is minimized: the runtime pushes the
    /// variables a turn leaves behind *before* it carries out that turn's
    /// actions, so a tile that named its file straight away named one that did
    /// not exist yet — and GTK, having failed to load it, remembers the failure
    /// and never looks again.
    Captured {
        addr: WindowAddr,
    },
    Finished {
        id: JobId,
    },
}
