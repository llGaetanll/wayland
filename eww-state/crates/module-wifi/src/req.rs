//! What the Wi-Fi module can ask its parent for.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReqWifi {
    /// A join finished, so whatever dropdown is showing this list should go.
    /// Whether there even is one is the parent's business, not ours.
    Dismiss,
}
