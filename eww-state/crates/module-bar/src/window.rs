//! The eww windows the bar owns.
//!
//! **Declaration order is stacking order.** The derived `Ord` decides how these
//! sort in the set the runtime reconciles against, which decides the order they
//! are passed to `eww open-many`, which decides which surface ends up on top —
//! eww stacks a later overlay above an earlier one.
//!
//! So every backdrop is declared *before* the panel it sits behind. Get that
//! backwards and the full-screen click-catcher covers the panel: every click
//! lands on the backdrop, the panel dismisses itself, and none of its buttons
//! can ever be pressed.

use crate::menu::MenuId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Window {
    Bar,
    Dock,
    /// Full-screen click-catcher shared by every bar dropdown.
    MenuBackdrop,
    Menu(MenuId),
    /// The screenshot preview's own click-catcher, deliberately separate so a
    /// click outside the preview leaves an open bar menu alone.
    ShotBackdrop,
    Shot,
    /// The launcher's own click-catcher, separate for the same reason the
    /// screenshot preview's is.
    SearchBackdrop,
    Search,
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
            Window::Search => "search".into(),
            Window::SearchBackdrop => "search-backdrop".into(),
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
        v.push(Window::Search);
        v.push(Window::SearchBackdrop);
        v
    }
}
