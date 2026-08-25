//! Volume: the bar icon, the slider popup, and the wpctl behind them.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionAudio;
pub use config::ConfigAudio;
pub use event::EventAudio;
pub use model::Audio;
pub use state::StateAudio;
