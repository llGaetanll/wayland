//! Everything that can move the machine forward.
//!
//! The machine has exactly one input: `Event`. Clicks, probe results and job
//! completions all arrive through here and are applied one at a time, in order,
//! which is what removes the interleaving the shell-script version suffered.

use crate::model::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Something the user asked for, from a widget handler or the CLI.
    Intent(Intent),
    /// A probe reported the world's current state.
    Observed(Obs),
    /// A side effect the machine dispatched has finished.
    JobDone {
        id: JobId,
        ok: bool,
        detail: Option<String>,
    },
    /// eww came back (daemon restart / reload). Everything must be republished.
    EwwReset,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// Open `menu` if closed, close it if already open, switch if another is up.
    MenuToggle(MenuId),
    MenuClose,
    VolumeSet(u8),
    BrightnessSet(u8),
    WifiToggle,
    WifiConnect {
        ssid: String,
    },
    BtToggle,
    /// Connect the device, or disconnect it when it is already connected.
    BtConnect {
        mac: String,
    },
    DockLaunch {
        id: String,
    },
    DockSwap {
        a: String,
        b: String,
    },
    /// Take a screenshot: region select, then stage it for preview.
    ShotCapture,
    /// A capture finished and left this file behind.
    ShotStage(Shot),
    ShotAct(ShotAction),
    /// Force a probe to run now, outside its interval.
    Refresh(Probe),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Obs {
    Theme(Theme),
    Clock(String),
    Battery(Battery),
    Audio(Audio),
    Backlight(u8),
    Net(NetSnapshot),
    Bt(BtSnapshot),
    Dock(Vec<DockApp>),
    /// True when the focused workspace holds a fullscreen window, so the bar
    /// and dock should get out of the way.
    Fullscreen(bool),
}

/// A thing the shell knows how to sample. The machine decides *when*, never how.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Probe {
    Clock,
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
        Probe::Clock,
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
            Probe::Clock => "clock",
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
/// action and matches it up again on `Event::JobDone`, so a result can never be
/// attributed to the wrong request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);
