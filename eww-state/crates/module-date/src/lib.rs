//! The date and time shown in the bar.
//!
//! Formatting is the runtime's job — a state machine with no IO has no clock,
//! and the tick that does reach one carries a duration, not a calendar. So the
//! already-formatted string arrives as an observation like any other.

pub mod action;
pub mod config;
pub mod event;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionDate;
pub use config::ConfigDate;
pub use event::EventDate;
pub use state::StateDate;
