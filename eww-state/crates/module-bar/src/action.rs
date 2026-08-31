//! Everything the bar can ask the outside world to do.
//!
//! Almost none of these are the bar's own: each variant is one module's action,
//! re-labelled on the way out so the runtime knows which module's IO to hand it
//! to — and so a completion can find its way home without anybody keeping a
//! map. The exception is [`ActionBar`], which is about eww itself, and so
//! belongs to no widget.

use module_audio::ActionAudio;
use module_backlight::ActionBacklight;
use module_battery::ActionBattery;
use module_bluetooth::ActionBluetooth;
use module_date::ActionDate;
use module_dock::ActionDock;
use module_shot::ActionShot;
use module_spaces::ActionSpaces;
use module_theme::ActionTheme;
use module_wifi::ActionWifi;

use crate::action_bar::ActionBar;

#[derive(Debug)]
pub enum Action {
    Bar(ActionBar),
    Audio(ActionAudio),
    Backlight(ActionBacklight),
    Battery(ActionBattery),
    Bluetooth(ActionBluetooth),
    Date(ActionDate),
    Dock(ActionDock),
    Shot(ActionShot),
    Spaces(ActionSpaces),
    Theme(ActionTheme),
    Wifi(ActionWifi),
}
