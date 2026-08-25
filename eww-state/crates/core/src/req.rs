//! What a sub-machine can ask of its parent.
//!
//! A machine never reaches sideways or upward: it says what it wants and the
//! parent decides. Returned from `emit_actions`, because wanting something of
//! the parent is a decision like any other and belongs where decisions are made.

/// Something only the parent can grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Req {
    /// Dismiss whatever bar dropdown is open.
    CloseMenu,
}
