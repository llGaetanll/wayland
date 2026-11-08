//! Turning readings into the text the panel shows.
//!
//! Here rather than in the widget because eww's expression language has no
//! division that rounds the way a person reads a clock, and a duration split
//! across two `yuck` expressions is a rule written in two places.

/// A duration as hours and minutes: `4h 31m`, or `18m` under the hour.
///
/// The minutes are zero-padded only when there are hours beside them, so a
/// column of these lines up without `18m` becoming the odd `0h 18m`.
pub fn hm(seconds: u64) -> String {
    let minutes = seconds / 60;
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m}m"),
        (h, m) => format!("{h}h {m:02}m"),
    }
}

/// Draw, to one decimal: `5.5 W`. A pack draws single-digit watts, so the
/// decimal is the difference between two readings rather than noise.
pub fn watts(milliwatts: u32) -> String {
    format!("{:.1} W", f64::from(milliwatts) / 1_000.0)
}
