//! The battery pill, and the panel behind it.
//!
//! Both pictures are SVGs this module has the runtime regenerate whenever the
//! drawing would change, and both filenames encode what they drew — so the path
//! changes exactly when the picture does, which is what makes eww and GTK
//! reload them, since new bytes at a cached path would not.
//!
//! The panel's list is the one thing here that is an estimate rather than a
//! reading. There is no per-process power meter to read on this machine —
//! `intel-rapl`'s `energy_uj` is root-only — so the ranking is CPU time over a
//! window, grouped per app. The ordering is real; a per-app wattage would not
//! be, which is why none is published.

pub mod action;
pub mod config;
pub mod event;
pub mod graph;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;
pub mod svg;
pub mod wire_fmt;

pub use action::ActionBattery;
pub use config::ConfigBattery;
pub use event::EventBattery;
pub use model::Battery;
pub use model::HistPoint;
pub use model::Proc;
pub use model::Reading;
pub use state::StateBattery;
