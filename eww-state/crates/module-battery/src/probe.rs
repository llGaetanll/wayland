//! Reading capacity and charge state straight out of sysfs.

use crate::event::EventBattery;
use crate::model::Battery;

pub fn sample() -> EventBattery {
    let read = |f: &str| std::fs::read_to_string(format!("/sys/class/power_supply/BAT0/{f}")).ok();
    let percent = read("capacity")
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0)
        .min(100);
    let status = read("status").unwrap_or_default();
    EventBattery::Observed {
        battery: Battery {
            percent,
            // "Full" draws the same as charging: the cable is in.
            charging: matches!(status.trim(), "Charging" | "Full"),
        },
    }
}
