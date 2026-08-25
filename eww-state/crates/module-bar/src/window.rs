//! The eww windows the bar owns.

use crate::menu::MenuId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Window {
    Bar,
    Dock,
    Menu(MenuId),
    /// Full-screen click-catcher shared by every bar dropdown.
    MenuBackdrop,
    Shot,
    /// The screenshot preview's own click-catcher, deliberately separate so a
    /// click outside the preview leaves an open bar menu alone.
    ShotBackdrop,
}

impl Window {
    pub fn name(self) -> String {
        match self {
            Window::Bar => "bar".into(),
            Window::Dock => "dock".into(),
            Window::Menu(m) => m.window(),
            Window::MenuBackdrop => "menu-backdrop".into(),
            Window::Shot => "shot-menu".into(),
            Window::ShotBackdrop => "shot-backdrop".into(),
        }
    }

    /// Every window the bar may ever open. The runtime closes all of these at
    /// startup before opening the desired set, so a leftover surface from a
    /// previous daemon cannot survive as an unreapable orphan.
    pub fn all() -> Vec<Window> {
        let mut v = vec![Window::Bar, Window::Dock, Window::MenuBackdrop];
        v.extend(MenuId::ALL.into_iter().map(Window::Menu));
        v.push(Window::Shot);
        v.push(Window::ShotBackdrop);
        v
    }
}
