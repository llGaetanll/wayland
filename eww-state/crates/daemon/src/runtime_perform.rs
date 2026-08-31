//! Handing each action to the module that owns it.
//!
//! This is the whole of the runtime's knowledge about widgets: which module a
//! variant belongs to. What the command is, how it is invoked, and what its
//! output means all live in that module, next to the state machine that asked.

use bar::Action;
use bar::Event;
use module_bar as bar;

use crate::paths::Paths;

/// The bits of the environment a module's IO needs and cannot discover.
pub struct Places {
    pub wifi: module_wifi::perform::Prompt,
    pub shot: module_shot::perform::Places,
    pub cache: std::path::PathBuf,
    pub dock_conf: std::path::PathBuf,
    pub date_format: String,
}

impl Places {
    pub fn new(paths: &Paths, accent: String, date_format: String) -> Places {
        Places {
            wifi: module_wifi::perform::Prompt {
                theme: format!(
                    "{}/rofi/themes/menu.rasi",
                    paths
                        .eww_config
                        .parent()
                        .map(Paths::str)
                        .unwrap_or_default()
                ),
            },
            shot: module_shot::perform::Places {
                staging: paths.shots.clone(),
                saved: paths.screenshots.clone(),
                accent,
                saved_format: "+Screenshot %Y-%m-%d %H.%M.%S.png".to_string(),
            },
            cache: paths.cache.clone(),
            dock_conf: paths.dock_conf.clone(),
            date_format,
        }
    }
}

/// Carry one action out, and hand back whatever it has to say.
pub fn perform(action: &Action, places: &Places) -> Vec<Event> {
    match action {
        // Carried out by the event loop instead: these are about eww and the
        // reconciler, which it owns.
        Action::Bar(_) => Vec::new(),
        Action::Audio(a) => module_audio::perform::perform(a)
            .into_iter()
            .map(Event::Audio)
            .collect(),
        Action::Backlight(a) => module_backlight::perform::perform(a)
            .into_iter()
            .map(Event::Backlight)
            .collect(),
        Action::Battery(a) => module_battery::perform::perform(a, &places.cache)
            .into_iter()
            .map(Event::Battery)
            .collect(),
        Action::Bluetooth(a) => module_bluetooth::perform::perform(a)
            .into_iter()
            .map(Event::Bluetooth)
            .collect(),
        Action::Date(a) => module_date::perform::perform(a, &places.date_format)
            .into_iter()
            .map(Event::Date)
            .collect(),
        Action::Dock(a) => module_dock::perform::perform(a, &places.dock_conf)
            .into_iter()
            .map(Event::Dock)
            .collect(),
        Action::Shot(a) => module_shot::perform::perform(a, &places.shot)
            .into_iter()
            .map(Event::Shot)
            .collect(),
        Action::Spaces(a) => module_spaces::perform::perform(a)
            .into_iter()
            .map(Event::Spaces)
            .collect(),
        Action::Theme(a) => module_theme::perform::perform(a)
            .into_iter()
            .map(Event::Theme)
            .collect(),
        Action::Wifi(a) => module_wifi::perform::perform(a, &places.wifi)
            .into_iter()
            .map(Event::Wifi)
            .collect(),
    }
}
