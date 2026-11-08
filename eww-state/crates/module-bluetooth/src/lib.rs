//! Bluetooth: the bar icon, the device dropdown, and bluetoothctl.
//!
//! Shaped like `module-wifi`, with one deliberate difference: acting on a
//! device leaves the menu open, because pairing takes long enough that watching
//! the row update is the point. That is why this module never asks to be
//! dismissed and Wi-Fi does.

pub mod action;
pub mod config;
pub mod event;
pub mod model;
pub mod perform;
pub mod probe;
pub mod state;

pub use action::ActionBluetooth;
pub use config::ConfigBluetooth;
pub use event::EventBluetooth;
pub use model::BtDevice;
pub use model::BtSnapshot;
pub use state::StateBluetooth;
