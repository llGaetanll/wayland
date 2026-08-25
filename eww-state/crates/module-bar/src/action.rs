//! Everything the bar can ask the outside world to do.
//!
//! The bar mints none of these itself except its own: each variant is one
//! module's action, re-labelled on the way out so the runtime knows which
//! module's IO to hand it to — and so a completion can find its way home
//! without anybody keeping a map.

use module_audio::ActionAudio;
use module_backlight::ActionBacklight;
use module_battery::ActionBattery;
use module_bluetooth::ActionBluetooth;
use module_date::ActionDate;
use module_dock::ActionDock;
use module_shot::ActionShot;
use module_theme::ActionTheme;
use module_wifi::ActionWifi;

#[derive(Debug)]
pub enum Action {
    /// Ask the compositor whether a fullscreen window is focused.
    SampleFullscreen,
    Audio(ActionAudio),
    Backlight(ActionBacklight),
    Battery(ActionBattery),
    Bluetooth(ActionBluetooth),
    Date(ActionDate),
    Dock(ActionDock),
    Shot(ActionShot),
    Theme(ActionTheme),
    Wifi(ActionWifi),
}
