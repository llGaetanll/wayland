//! Asking `date(1)` what time it is.

use crate::event::EventDate;

pub fn sample(format: &str) -> EventDate {
    EventDate::Observed {
        text: exec::out("date", &[format]).unwrap_or_default(),
    }
}
