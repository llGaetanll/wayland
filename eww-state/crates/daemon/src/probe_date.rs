//! The date and time, already formatted the way the bar shows it.

use eww_state_core::EventProbe;

use crate::proc;

pub fn sample() -> EventProbe {
    EventProbe::Date {
        text: proc::out("date", &["+%a %b %e  %H:%M"]).unwrap_or_default(),
    }
}
