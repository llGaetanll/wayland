//! The bar: the root state machine, and the only thing that knows every module.
//!
//! Nothing here decides how Wi-Fi works or what a battery pill looks like. It
//! routes an event to the one module it belongs to, re-labels that module's
//! actions on the way out, and owns the one fact no module can own alone:
//! which dropdown is open.
//!
//! Adding a widget is a new `module-*` crate, one variant in [`Event`], one in
//! [`Action`], and one line in the routing. It is not a change to any other
//! module.

pub mod action;
pub mod action_bar;
pub mod config;
pub mod event;
pub mod event_bar;
pub mod event_clock;
pub mod eww_gen;
pub mod menu;
pub mod module_id;
pub mod server_id;
pub mod state;
pub mod window;

pub use action::Action;
pub use action_bar::ActionBar;
pub use config::ConfigBar;
pub use event::Event;
pub use event_bar::EventBar;
pub use event_clock::EventClock;
pub use eww_gen::EwwGen;
pub use menu::MenuId;
pub use module_id::ModuleId;
pub use module_id::PollPlan;
pub use server_id::ServerId;
pub use state::StateBar;
pub use state::EWW_GEN_VAR;
pub use window::Window;
