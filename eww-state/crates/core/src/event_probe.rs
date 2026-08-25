//! What the world currently looks like.
//!
//! One variant per probe, carrying the parsed snapshot. `Due` is the other
//! direction — a timer fired, the compositor moved, or someone asked for a
//! resample — and is the only reason the machine ever emits a probe action.

use crate::model::Audio;
use crate::model::Battery;
use crate::model::BtSnapshot;
use crate::model::DockApp;
use crate::model::NetSnapshot;
use crate::model::Theme;
use crate::probe::Probe;

#[derive(Debug)]
pub enum EventProbe {
    /// This probe is due now, outside its interval.
    Due {
        what: Probe,
    },
    Theme {
        theme: Theme,
    },
    Date {
        text: String,
    },
    Battery {
        battery: Battery,
    },
    Audio {
        audio: Audio,
    },
    Backlight {
        percent: u8,
    },
    Net {
        net: NetSnapshot,
    },
    Bt {
        bt: BtSnapshot,
    },
    Dock {
        apps: Vec<DockApp>,
    },
    /// True when the focused workspace holds a fullscreen window, so the bar
    /// and dock should get out of the way.
    Fullscreen {
        on: bool,
    },
}
