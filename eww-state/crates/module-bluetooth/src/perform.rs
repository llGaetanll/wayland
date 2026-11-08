//! Carrying out what the Bluetooth module asked for.

use crate::action::ActionBluetooth;
use crate::event::EventBluetooth;
use crate::probe;

pub fn perform(action: &ActionBluetooth) -> Option<EventBluetooth> {
    match action {
        ActionBluetooth::Sample => Some(probe::sample()),
        ActionBluetooth::SetPowered { id, on } => {
            let ok = exec::ok("bluetoothctl", &["power", if *on { "on" } else { "off" }]);
            Some(EventBluetooth::Finished { id: *id, ok })
        }
        // Fire and forget: nothing waits on discovery.
        ActionBluetooth::Scan => {
            exec::detach("bluetoothctl --timeout 10 scan on >/dev/null 2>&1");
            None
        }
        ActionBluetooth::Connect { id, mac } => {
            // Trust first so it reconnects by itself next time. No explicit
            // `pair`: BlueZ completes any missing pairing during connect, and
            // `pair` fails outright on a device not in pairing mode.
            exec::ok("bluetoothctl", &["trust", mac]);
            let ok = exec::ok("bluetoothctl", &["connect", mac]);
            Some(EventBluetooth::Finished { id: *id, ok })
        }
        ActionBluetooth::Disconnect { id, mac } => {
            let ok = exec::ok("bluetoothctl", &["disconnect", mac]);
            Some(EventBluetooth::Finished { id: *id, ok })
        }
        ActionBluetooth::Notify { body } => {
            exec::ok("notify-send", &["Bluetooth", body]);
            None
        }
    }
}
