//! What the Spaces module can ask the outside world to do.

use api::JobId;

use crate::model::MoveTarget;
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
    /// Go to the workspace a window came from, before closing it: a Space that
    /// is emptied while still focused lingers instead of being destroyed.
    Focus {
        workspace: WorkspaceId,
    },
    Close {
        addr: WindowAddr,
    },
}
