//! What the shell knows how to sample.
//!
//! The machine decides *when* a probe runs, never how. Naming one is the whole
//! extent of its knowledge about the outside world.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Probe {
    Date,
    Battery,
    Theme,
    Audio,
    Backlight,
    Net,
    Bt,
    Dock,
    Fullscreen,
}

impl Probe {
    pub const ALL: [Probe; 9] = [
        Probe::Date,
        Probe::Battery,
        Probe::Theme,
        Probe::Audio,
        Probe::Backlight,
        Probe::Net,
        Probe::Bt,
        Probe::Dock,
        Probe::Fullscreen,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Probe::Date => "date",
            Probe::Battery => "battery",
            Probe::Theme => "theme",
            Probe::Audio => "audio",
            Probe::Backlight => "backlight",
            Probe::Net => "net",
            Probe::Bt => "bt",
            Probe::Dock => "dock",
            Probe::Fullscreen => "fullscreen",
        }
    }

    pub fn parse(s: &str) -> Option<Probe> {
        Probe::ALL.into_iter().find(|p| p.as_str() == s)
    }
}

/// Handle for a dispatched side effect. The machine hands one out with every
/// action and matches it up again when the job reports back, so a result can
/// never be attributed to the wrong request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);
