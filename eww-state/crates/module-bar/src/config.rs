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

#[derive(Clone, Debug, Default)]
pub struct ConfigBar {
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
            audio: ConfigAudio {
                icons_dir: icons_dir.to_string(),
                ..Default::default()
            },
            backlight: ConfigBacklight {
                icons_dir: icons_dir.to_string(),
                ..Default::default()
            },
            battery: ConfigBattery {
                cache_dir: cache_dir.to_string(),
                ..Default::default()
            },
            bluetooth: ConfigBluetooth {
                icons_dir: icons_dir.to_string(),
                ..Default::default()
            },
            wifi: ConfigWifi {
                icons_dir: icons_dir.to_string(),
                ..Default::default()
            },
            ..ConfigBar::default()
        }
    }
}
