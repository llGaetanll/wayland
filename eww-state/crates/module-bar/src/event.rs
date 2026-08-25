//! Everything that can move the bar forward.
//!
//! One variant per module, because that is what the bar routes on. Where an
//! event *came from* — a click, a sample, a job reporting back — is said by the
//! variant name inside each module's own alphabet, which is where it matters.

use module_audio::EventAudio;
use module_backlight::EventBacklight;
use module_battery::EventBattery;
use module_bluetooth::EventBluetooth;
use module_date::EventDate;
use module_dock::EventDock;
use module_shot::EventShot;
use module_theme::EventTheme;
use module_wifi::EventWifi;

use crate::event_bar::EventBar;
use crate::event_clock::EventClock;

#[derive(Debug)]
pub enum Event {
    Bar(EventBar),
    /// Fanned out to every module with something to give up on.
    Clock(EventClock),
    Audio(EventAudio),
    Backlight(EventBacklight),
    Battery(EventBattery),
    Bluetooth(EventBluetooth),
    Date(EventDate),
    Dock(EventDock),
    Shot(EventShot),
    Theme(EventTheme),
    Wifi(EventWifi),
}
