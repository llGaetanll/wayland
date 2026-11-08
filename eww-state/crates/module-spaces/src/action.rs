//! What the Spaces module can ask the outside world to do.

use api::JobId;

use crate::model::MoveTarget;
use crate::model::Rect;
use crate::model::WindowAddr;
use crate::model::WorkspaceId;

#[derive(Debug)]
pub enum ActionSpaces {
    /// Ask the compositor what its layout is now.
    Look {
        id: JobId,
    },
    /// Send a window to its own Space, or back where it came from. That the cut
    /// has to be invisible is the runtime's problem, not this module's.
    Move {
        id: JobId,
        addr: WindowAddr,
        to: MoveTarget,
    },
    /// Photograph a window and then hide it, in that order and as one action.
    ///
    /// Not two, even though the second is an ordinary [`Move`](Self::Move):
    /// every action is carried out on a thread of its own, so a capture and a
    /// move emitted together would race, and the race is lost about half the
    /// time — with nothing left on screen to photograph.
    Minimize {
        id: JobId,
        addr: WindowAddr,
        geometry: Rect,
        /// Where to write the capture.
        thumb: String,
        to: MoveTarget,
    },
    /// Go to the workspace a window came from, before closing it: a Space that
    /// is emptied while still focused lingers instead of being destroyed.
    Focus {
        workspace: WorkspaceId,
    },
    Close {
        addr: WindowAddr,
    },
    /// Drop a thumbnail: its window is back on screen, or gone for good.
    Forget {
        thumb: String,
    },
}
