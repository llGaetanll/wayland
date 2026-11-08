//! The dropdowns hanging off the top-bar status icons.

/// These five are the whole set: one slot holds at most one of them, which is
/// what makes "two menus open at once" a state that does not exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MenuId {
    Wifi,
    Bt,
    Volume,
    Brightness,
    Battery,
}

impl MenuId {
    pub const ALL: [MenuId; 5] = [
        MenuId::Wifi,
        MenuId::Bt,
        MenuId::Volume,
        MenuId::Brightness,
        MenuId::Battery,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            MenuId::Wifi => "wifi",
            MenuId::Bt => "bt",
            MenuId::Volume => "volume",
            MenuId::Brightness => "brightness",
            MenuId::Battery => "battery",
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

impl std::fmt::Display for MenuId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
