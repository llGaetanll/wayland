//! Bluetooth adapter and device state, from bluetoothctl.

use eww_state_core::BtDevice;
use eww_state_core::BtSnapshot;
use eww_state_core::EventProbe;

use crate::proc;

pub fn sample() -> EventProbe {
    let powered = proc::out("bluetoothctl", &["show"])
        .map(|o| parse_powered(&o))
        .unwrap_or(false);
    if !powered {
        return EventProbe::Bt {
            bt: BtSnapshot::default(),
        };
    }
    // No single list is reliable: bluetoothctl reports some known devices as
    // Trusted with Paired/Bonded = no. The union, deduped, is "My Devices".
    let mut listing = String::new();
    for kind in ["Paired", "Bonded", "Trusted", "Connected"] {
        if let Some(o) = proc::out("bluetoothctl", &["devices", kind]) {
            listing.push_str(&o);
            listing.push('\n');
        }
    }
    let connected = proc::out("bluetoothctl", &["devices", "Connected"]).unwrap_or_default();
    EventProbe::Bt {
        bt: BtSnapshot {
            powered,
            devices: parse_devices(&listing, &connected),
        },
    }
}

fn parse_powered(show: &str) -> bool {
    show.lines()
        .find_map(|l| l.trim().strip_prefix("Powered:"))
        .map(|v| v.trim() == "yes")
        .unwrap_or(false)
}

/// `bluetoothctl devices` prints `Device AA:BB:CC:DD:EE:FF Name With Spaces`.
fn parse_devices(listing: &str, connected: &str) -> Vec<BtDevice> {
    let connected_macs: Vec<&str> = connected
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    let mut devices: Vec<BtDevice> = Vec::new();
    for line in listing.lines() {
        let mut parts = line.splitn(3, ' ');
        if parts.next() != Some("Device") {
            continue;
        }
        let Some(mac) = parts.next() else { continue };
        if devices.iter().any(|d| d.mac == mac) {
            continue;
        }
        devices.push(BtDevice {
            mac: mac.to_string(),
            name: parts.next().unwrap_or(mac).to_string(),
            connected: connected_macs.contains(&mac),
        });
    }
    devices
}

#[cfg(test)]
mod tests {
    use super::parse_devices;
    use super::parse_powered;

    #[test]
    fn the_union_is_deduped_and_names_keep_spaces() {
        let listing =
            "Device AA:BB Sony WH-1000XM4\nDevice AA:BB Sony WH-1000XM4\nDevice CC:DD Mouse";
        let devices = parse_devices(listing, "Device CC:DD Mouse");
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Sony WH-1000XM4");
        assert!(!devices[0].connected);
        assert!(devices[1].connected);
    }

    #[test]
    fn powered_flag() {
        assert!(parse_powered(
            "Controller X\n\tPowered: yes\n\tDiscoverable: no"
        ));
        assert!(!parse_powered("Controller X\n\tPowered: no"));
        assert!(!parse_powered(""));
    }
}
