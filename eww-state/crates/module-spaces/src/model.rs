//! The compositor's window layout, in our own words.

/// A window's address, canonically `0x…`.
///
/// Kept as a type rather than a string because every one of these comes from
/// the compositor and goes back out in a dispatch; the parsing is where the two
/// spellings it uses are reconciled, and nowhere else has to know.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowAddr(String);

impl WindowAddr {
    /// `hyprctl clients` prints an address bare, `lastwindow` prints it with an
    /// `0x`, and a dispatch accepts only the second.
    ///
    /// `0x0` is how the compositor says *no window*, so it is `None` rather
    /// than a window whose address happens to be zero.
    pub fn parse(s: &str) -> Option<WindowAddr> {
        let hex = s.trim();
        let hex = hex.strip_prefix("0x").unwrap_or(hex);
        let known = !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
        if !known || hex.chars().all(|c| c == '0') {
            return None;
        }
        Some(WindowAddr(format!("0x{hex}")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WindowAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceId(pub i32);

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One window, as much of it as any rule here cares about.
#[derive(Clone, Debug)]
pub struct Client {
    pub addr: WindowAddr,
    pub workspace: WorkspaceId,
    /// The compositor reports a mode, not a flag — maximized and fullscreen are
    /// different numbers. Both count: either one is the user asking for the
    /// window to fill the screen, which is what a Space is for.
    pub fullscreen: bool,
    /// Which monitor it is on. Nothing reads it yet — there is one screen — but
    /// a rule that comes to need it should have to say so, rather than inherit
    /// an assumption nobody wrote down.
    pub monitor: i32,
}

/// Everything one look at the compositor found.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub focused: WorkspaceId,
    /// The focused window, if the focused workspace has one.
    pub active: Option<WindowAddr>,
    pub clients: Vec<Client>,
}

impl Snapshot {
    pub fn client(&self, addr: &WindowAddr) -> Option<&Client> {
        self.clients.iter().find(|c| &c.addr == addr)
    }

    /// The focused workspace holds a fullscreen window.
    pub fn fullscreen(&self) -> bool {
        self.clients
            .iter()
            .any(|c| c.workspace == self.focused && c.fullscreen)
    }
}

/// Where a window is being moved.
#[derive(Debug)]
pub enum MoveTarget {
    /// A Space of its own: the compositor picks the first workspace with
    /// nothing on it.
    Empty,
    /// Back where it came from. Emptying the Space is what makes the compositor
    /// destroy it.
    Workspace(WorkspaceId),
}

#[cfg(test)]
mod tests {
    use super::WindowAddr;

    #[test]
    fn an_address_has_one_spelling_however_it_arrives() {
        let bare = WindowAddr::parse("558156ab0980").expect("a bare address");
        let prefixed = WindowAddr::parse("0x558156ab0980").expect("a prefixed address");
        assert_eq!(bare, prefixed);
        assert_eq!(bare.as_str(), "0x558156ab0980", "a dispatch needs the 0x");
    }

    #[test]
    fn nothing_is_not_a_window() {
        assert_eq!(WindowAddr::parse("0x0"), None);
        assert_eq!(WindowAddr::parse(""), None);
        assert_eq!(WindowAddr::parse("(none)"), None);
    }
}
