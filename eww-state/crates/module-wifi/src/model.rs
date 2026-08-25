//! What the world looks like, as far as Wi-Fi is concerned.

/// One network, as the bar draws it.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secure: bool,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct NetSnapshot {
    /// Wi-Fi radio state.
    pub enabled: bool,
    /// SSID of the network currently in use, if any.
    pub connected: Option<String>,
    /// One entry per SSID, strongest first.
    pub networks: Vec<WifiNetwork>,
    /// A wired link is up; the bar shows the ethernet icon when Wi-Fi is idle.
    pub wired: bool,
}

impl NetSnapshot {
    /// Signal strength of the network in use, if connected over Wi-Fi.
    pub fn active_signal(&self) -> Option<u8> {
        self.networks.iter().find(|n| n.active).map(|n| n.signal)
    }
}
