//! Battery capacity and charge state, read straight from sysfs.

use eww_state_core::Battery;
use eww_state_core::EventProbe;

pub fn sample() -> EventProbe {
    let read = |f: &str| std::fs::read_to_string(format!("/sys/class/power_supply/BAT0/{f}")).ok();
    let percent = read("capacity")
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0)
        .min(100);
    let status = read("status").unwrap_or_default();
    EventProbe::Battery {
        battery: Battery {
            percent,
            // "Full" draws the same as charging: the cable is in.
            charging: matches!(status.trim(), "Charging" | "Full"),
        },
    }
}
