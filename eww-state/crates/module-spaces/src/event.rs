//! Everything that can move the Spaces module forward.

use api::JobId;

use crate::model::Snapshot;

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
    Finished {
        id: JobId,
    },
}
