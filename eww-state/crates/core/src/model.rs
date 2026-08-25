//! Plain data shared by events, state and effects.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

/// A dropdown hanging off a top-bar status icon. These four are the whole set:
/// one `menu` slot holds at most one of them, which is what makes "two menus
/// open at once" unrepresentable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MenuId {
    Wifi,
    Bt,
    Volume,
    Brightness,
}

impl MenuId {
    pub const ALL: [MenuId; 4] = [MenuId::Wifi, MenuId::Bt, MenuId::Volume, MenuId::Brightness];

    pub fn as_str(self) -> &'static str {
        match self {
            MenuId::Wifi => "wifi",
            MenuId::Bt => "bt",
            MenuId::Volume => "volume",
            MenuId::Brightness => "brightness",
        }
    }

    pub fn parse(s: &str) -> Option<MenuId> {
        MenuId::ALL.into_iter().find(|m| m.as_str() == s)
    }

    /// eww window name for this menu's panel.
    pub fn window(self) -> String {
        format!("{}-menu", self.as_str())
    }
}

impl fmt::Display for MenuId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct NetSnapshot {
    /// Wi-Fi radio state.
    pub enabled: bool,
    /// SSID of the network currently in use, if any.
    pub connected: Option<String>,
    /// One entry per SSID, strongest first.
    pub networks: Vec<WifiNetwork>,
    /// A wired link is up; the bar shows the ethernet icon when Wi-Fi is idle.
    pub wired: bool,
}

impl NetSnapshot {
    /// Signal strength of the network in use, if connected over Wi-Fi.
    pub fn active_signal(&self) -> Option<u8> {
        self.networks.iter().find(|n| n.active).map(|n| n.signal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BtSnapshot {
    pub powered: bool,
    pub devices: Vec<BtDevice>,
}

impl BtSnapshot {
    pub fn connected_name(&self) -> Option<&str> {
        self.devices
            .iter()
            .find(|d| d.connected)
            .map(|d| d.name.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Audio {
    pub volume: u8,
    pub muted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// One dock entry. The registry and the order are the same list, so there is no
/// second file to keep in sync.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockApp {
    pub id: String,
    pub cmd: String,
    pub icon: String,
}

/// A staged screenshot awaiting Copy / Save / Discard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    pub path: String,
    /// Preview box size, already fitted to the capture's aspect ratio.
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotAction {
    Copy,
    Save,
    Discard,
    Dismiss,
}

impl ShotAction {
    pub fn parse(s: &str) -> Option<ShotAction> {
        Some(match s {
            "copy" => ShotAction::Copy,
            "save" => ShotAction::Save,
            "discard" => ShotAction::Discard,
            "close" | "dismiss" => ShotAction::Dismiss,
            _ => return None,
        })
    }
}
