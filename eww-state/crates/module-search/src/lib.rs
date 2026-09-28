//! The application launcher: the panel `SUPER + Space` puts up.
//!
//! Its own module rather than a bar dropdown, for the reason the screenshot
//! preview is: it is a keybind popup, so it must neither dismiss an open bar
//! menu nor be dismissed by one.
//!
//! The one thing here that no other module needs is a keyboard. eww has no key
//! bindings, so the text field is eww's own `input` widget and everything it
//! will not handle — Escape, the arrow keys — reaches us through a compositor
//! submap. See [`action::ActionSearch::Keys`].

pub mod action;
pub mod config;
pub mod event;
pub mod matcher;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionSearch;
pub use config::ConfigSearch;
pub use event::EventSearch;
pub use model::App;
pub use state::StateSearch;
