//! Naming a module from outside it.
//!
//! The runtime needs to say "this one is due" without knowing what any of them
//! do, and the bar needs to turn that back into the right module's event.

use std::collections::BTreeMap;

use module_audio::EventAudio;
use module_backlight::EventBacklight;
use module_battery::EventBattery;
use module_bluetooth::EventBluetooth;
use module_date::EventDate;
use module_dock::EventDock;
use module_spaces::EventSpaces;
use module_theme::EventTheme;
use module_wifi::EventWifi;

use crate::event::Event;

/// Interval, in milliseconds, at which a module wants sampling. A module absent
/// from the plan is never sampled on a timer.
pub type PollPlan = BTreeMap<ModuleId, u32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleId {
    Audio,
    Backlight,
    Battery,
    Bluetooth,
    Date,
    Dock,
    /// Where fullscreen windows go. Event-driven off the compositor rather
    /// than sampled on a timer, so it is in the plan only to be asked.
    Spaces,
    Theme,
    Wifi,
}

impl ModuleId {
    pub const ALL: [ModuleId; 9] = [
        ModuleId::Audio,
        ModuleId::Backlight,
        ModuleId::Battery,
        ModuleId::Bluetooth,
        ModuleId::Date,
        ModuleId::Dock,
        ModuleId::Spaces,
        ModuleId::Theme,
        ModuleId::Wifi,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ModuleId::Audio => "audio",
            ModuleId::Backlight => "backlight",
            ModuleId::Battery => "battery",
            ModuleId::Bluetooth => "bt",
            ModuleId::Date => "date",
            ModuleId::Dock => "dock",
            ModuleId::Spaces => "spaces",
            ModuleId::Theme => "theme",
            ModuleId::Wifi => "wifi",
        }
    }

    pub fn parse(s: &str) -> Option<ModuleId> {
        ModuleId::ALL.into_iter().find(|m| m.as_str() == s)
    }

    /// "You are due" in the module's own words. The runtime knows when, and
    /// nothing else; this is where that becomes an event someone can act on.
    pub fn due(self) -> Event {
        match self {
            ModuleId::Audio => Event::Audio(EventAudio::Due),
            ModuleId::Backlight => Event::Backlight(EventBacklight::Due),
            ModuleId::Battery => Event::Battery(EventBattery::Due),
            ModuleId::Bluetooth => Event::Bluetooth(EventBluetooth::Due),
            ModuleId::Date => Event::Date(EventDate::Due),
            ModuleId::Dock => Event::Dock(EventDock::Due),
            ModuleId::Spaces => Event::Spaces(EventSpaces::Due),
            ModuleId::Theme => Event::Theme(EventTheme::Due),
            ModuleId::Wifi => Event::Wifi(EventWifi::Due),
        }
    }
}
