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

/// A window's place on the screen, in compositor pixels.
///
/// The client area only: a title bar the compositor draws itself sits above
/// `y`, so a thumbnail taken from this shows the window's contents without its
/// chrome — which is what macOS shows too.
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// One window, as much of it as any rule here cares about.
#[derive(Clone, Debug)]
pub struct Client {
    pub addr: WindowAddr,
    pub workspace: WorkspaceId,
    /// What the compositor calls the workspace. Never interpreted, with one
    /// exception: the workspace minimized windows are kept on is ours, and is
    /// recognised by name because the compositor assigns special workspaces an
    /// id we cannot predict.
    pub workspace_name: String,
    /// The compositor reports a mode, not a flag — maximized and fullscreen are
    /// different numbers. Both count: either one is the user asking for the
    /// window to fill the screen, which is what a Space is for.
    pub fullscreen: bool,
    /// Whether the window floats over the layout rather than being tiled into
    /// it. The dock sits in front of a tiled window's bottom edge and behind a
    /// floating one, which is the only reason this is here.
    pub floating: bool,
    /// Which monitor it is on. Nothing reads it yet: where a Space is opened
    /// is the compositor's own answer to that question, asked by the monitor
    /// relative workspace selector in `perform`.
    pub monitor: i32,
    pub geometry: Rect,
    pub class: String,
    pub title: String,
}

/// One window put away in the dock.
///
/// Everything here is copied off the window at the moment it was minimized,
/// because none of it can be asked for afterwards: a hidden window's geometry
/// is the compositor's business, and its title is out of date the moment it
/// stops being drawn. The tile shows what the window looked like when it left,
/// which is exactly what macOS shows.
#[derive(Clone, Debug)]
pub struct Minimized {
    pub addr: WindowAddr,
    /// The workspace it came from, and goes back to.
    pub home: WorkspaceId,
    pub title: String,
    /// Where the capture of the window is, and whether it has got there yet.
    /// Published only once it has: see
    /// [`EventSpaces::Captured`](crate::event::EventSpaces::Captured).
    pub thumb: String,
    pub captured: bool,
    /// How wide to draw the tile, at the dock's tile height. The window's
    /// aspect ratio, in other words, which the widget cannot work out for
    /// itself: GTK sizes a box before it knows anything about the image it is
    /// about to paint into it.
    pub width: u32,
    /// The dock icon of the app it belongs to, badged on the thumbnail.
    /// Empty when the app is not in the dock, which the widget draws as no
    /// badge rather than as a missing image.
    pub icon: String,
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

    /// The focused workspace holds a tiled window, so something on screen has
    /// an edge the dock would cover.
    pub fn tiled(&self) -> bool {
        self.clients
            .iter()
            .any(|c| c.workspace == self.focused && !c.floating)
    }
}

/// Where a window is being moved.
#[derive(Debug)]
pub enum MoveTarget {
    /// A Space of its own: the compositor picks the first workspace with
    /// nothing on it, on the monitor the window is already on.
    Empty,
    /// Back where it came from. Emptying the Space is what makes the compositor
    /// destroy it.
    Workspace(WorkspaceId),
    /// Out of sight. Named rather than numbered: it is a special workspace, and
    /// the compositor picks those an id we never see coming.
    Hidden(String),
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
