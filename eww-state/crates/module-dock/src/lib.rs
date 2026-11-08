//! The dock: a list of apps, reorderable by dragging one onto another.
//!
//! The list *is* the order — there is no separate registry to fall out of sync
//! with it — and the file on disk is written from this state, never read back
//! into it except at startup.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionDock;
pub use config::ConfigDock;
pub use event::EventDock;
pub use model::DockApp;
pub use state::StateDock;
