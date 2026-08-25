//! Which sub-machine a job belongs to.

/// Which sub-machine a dispatched job belongs to, so its completion can be
/// routed back to the same place without the machines knowing about each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateId {
    Audio,
    Backlight,
    Wifi,
    Bluetooth,
    Battery,
    Date,
    Theme,
    Dock,
    Shot,
    Bar,
}
