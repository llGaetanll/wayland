//! Every module's configuration, gathered in one place.
//!
//! The bar holds this only long enough to hand each piece to the module it
//! belongs to; nothing reads another module's settings.

use module_audio::ConfigAudio;
use module_backlight::ConfigBacklight;
use module_battery::ConfigBattery;
use module_bluetooth::ConfigBluetooth;
use module_date::ConfigDate;
use module_dock::ConfigDock;
use module_shot::ConfigShot;
use module_theme::ConfigTheme;
use module_wifi::ConfigWifi;

/// How much larger than macOS's own point sizes to draw everything.
///
/// The capture machine is a 2x Retina Mac where one point is about two physical
/// pixels; this panel is 1920x1200 over ~14.5" at scale 1.0, so one point is one
/// physical pixel and every surface comes out roughly 1.4x physically smaller
/// than the same design on the Mac.
///
/// KEEP IN SYNC with `$ui-scale` in ~/.config/eww/metrics.scss and with the
/// window geometry literals in eww.yuck. Three copies, because none of the
/// three can read the others: CSS cannot size a scroll area in rows, and eww
/// resolves `defwindow` geometry statically, so a variable there fails to
/// open the window with "Unknown variable".
pub const UI_SCALE: f64 = 1.25;

/// Scale a measured macOS point size for this display.
pub fn scaled(points: u32) -> u32 {
    (points as f64 * UI_SCALE).round() as u32
}

#[derive(Clone, Debug, Default)]
pub struct ConfigBar {
    /// The root the light/dark icon sets live under. The modules are each
    /// pointed at one of its subdirectories, and re-pointed when the desktop's
    /// preference changes.
    pub icons_root: String,
    pub audio: ConfigAudio,
    pub backlight: ConfigBacklight,
    pub battery: ConfigBattery,
    pub bluetooth: ConfigBluetooth,
    pub date: ConfigDate,
    pub dock: ConfigDock,
    pub shot: ConfigShot,
    pub theme: ConfigTheme,
    pub wifi: ConfigWifi,
}

impl ConfigBar {
    /// The usual arrangement: icons in one directory, generated assets in
    /// another, everything else at its default.
    pub fn new(icons_dir: &str, cache_dir: &str) -> ConfigBar {
        ConfigBar {
            icons_root: icons_dir.to_string(),
            audio: ConfigAudio {
                icons_dir: format!("{icons_dir}/light"),
                ..Default::default()
            },
            backlight: ConfigBacklight {
                icons_dir: format!("{icons_dir}/light"),
                ..Default::default()
            },
            battery: ConfigBattery {
                cache_dir: cache_dir.to_string(),
                ..Default::default()
            },
            bluetooth: ConfigBluetooth {
                icons_dir: format!("{icons_dir}/light"),
                row_height: scaled(ConfigBluetooth::default().row_height),
                max_list_height: scaled(ConfigBluetooth::default().max_list_height),
                ..Default::default()
            },
            wifi: ConfigWifi {
                icons_dir: format!("{icons_dir}/light"),
                row_height: scaled(ConfigWifi::default().row_height),
                max_list_height: scaled(ConfigWifi::default().max_list_height),
                ..Default::default()
            },
            ..ConfigBar::default()
        }
    }
}
