//! Wi-Fi: the bar icon, the network dropdown, and the nmcli behind them.
//!
//! Everything about Wi-Fi in this program is in this crate and nothing else is.
//! The decisions are in [`StateWifi`], which does no IO; running `nmcli` is in
//! [`probe`] and [`perform`], which decide nothing. They live under one roof
//! because they are one subject, and they stay apart because they are two jobs.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod req;
pub mod state;

pub use action::ActionWifi;
pub use config::ConfigWifi;
pub use event::EventWifi;
pub use model::NetSnapshot;
pub use model::WifiNetwork;
pub use req::ReqWifi;
pub use state::StateWifi;
