//! The battery pill.
//!
//! The pill is an SVG this module has the runtime regenerate whenever the
//! drawing would change. Its filename encodes capacity and charge state, so the
//! path changes exactly when the picture does — which is what makes eww and GTK
//! reload it, since new bytes at a cached path would not.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;
pub mod svg;

pub use action::ActionBattery;
pub use config::ConfigBattery;
pub use event::EventBattery;
pub use model::Battery;
pub use state::StateBattery;
