//! A staged screenshot, and what can be done with it.

/// A capture awaiting Copy / Save / Discard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    pub path: String,
    /// The captured region's pixel size, before the preview is fitted to it.
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotAction {
    Copy,
    Save,
    Discard,
    Dismiss,
}

impl ShotAction {
    pub fn parse(s: &str) -> Option<ShotAction> {
        Some(match s {
            "copy" => ShotAction::Copy,
            "save" => ShotAction::Save,
            "discard" => ShotAction::Discard,
            "close" | "dismiss" => ShotAction::Dismiss,
            _ => return None,
        })
    }
}
