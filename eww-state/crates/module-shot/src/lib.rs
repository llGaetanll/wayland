//! The screenshot preview: capture, then Copy / Save / Discard.
//!
//! Deliberately independent of the bar dropdowns. A Print press must not
//! dismiss an open bar menu, and clicking outside the preview must not dismiss
//! one either, so this module owns its own window and its own backdrop.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod state;

pub use action::ActionShot;
pub use config::ConfigShot;
pub use event::EventShot;
pub use model::Shot;
pub use model::ShotAction;
pub use state::StateShot;
