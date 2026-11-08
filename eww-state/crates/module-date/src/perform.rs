//! Carrying out what the date module asked for.

use crate::action::ActionDate;
use crate::event::EventDate;
use crate::probe;

pub fn perform(action: &ActionDate, format: &str) -> Option<EventDate> {
    match action {
        ActionDate::Sample => Some(probe::sample(format)),
    }
}
