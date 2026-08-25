//! What the world looks like, as far as Bluetooth is concerned.

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BtSnapshot {
    pub powered: bool,
    pub devices: Vec<BtDevice>,
}

impl BtSnapshot {
    pub fn connected_name(&self) -> Option<&str> {
        self.devices
            .iter()
            .find(|d| d.connected)
            .map(|d| d.name.as_str())
    }
}
