//! Volume: the bar icon, the slider popup, the output list, and the wpctl and
//! pactl behind them.

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
pub use model::AudioOutputs;
pub use model::AudioSink;
pub use model::SinkKind;
pub use state::StateAudio;
