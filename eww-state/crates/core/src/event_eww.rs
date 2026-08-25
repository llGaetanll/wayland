//! What the eww daemon had to say about itself.

#[derive(Debug)]
pub enum EventEww {
    /// eww came back after a restart or a reload. Its vars are at their
    /// defaults and its windows are gone, so everything must be republished —
    /// which needs no state change here, only a fresh reconcile.
    Restarted,
}
