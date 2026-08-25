//! Sampling the world.
//!
//! Every probe is split in two: a pure `parse` function that turns a program's
//! output into an observation, and a thin `sample` that runs the program. The
//! parsers carry all the fiddly knowledge — nmcli's escaping, wpctl's mute
//! marker — and are unit-tested beside themselves without running anything.
//!
//! This file is the dispatch and nothing else; one probe per module.

use eww_state_core::EventProbe;
use eww_state_core::Probe;

use crate::hypr;
use crate::paths::Paths;
use crate::probe_audio;
use crate::probe_backlight;
use crate::probe_battery;
use crate::probe_bt;
use crate::probe_date;
use crate::probe_dock;
use crate::probe_net;
use crate::probe_theme;

pub fn sample(probe: Probe, paths: &Paths) -> Option<EventProbe> {
    match probe {
        Probe::Date => Some(probe_date::sample()),
        Probe::Battery => Some(probe_battery::sample()),
        Probe::Theme => Some(probe_theme::sample()),
        Probe::Audio => probe_audio::sample(),
        Probe::Backlight => probe_backlight::sample(),
        Probe::Net => Some(probe_net::sample()),
        Probe::Bt => Some(probe_bt::sample()),
        Probe::Dock => Some(probe_dock::sample(paths)),
        Probe::Fullscreen => hypr::sample(),
    }
}
