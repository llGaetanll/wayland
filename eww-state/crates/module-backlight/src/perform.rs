//! Carrying out what the brightness module asked for.

use crate::action::ActionBacklight;
use crate::event::EventBacklight;
use crate::probe;

pub fn perform(action: &ActionBacklight) -> Option<EventBacklight> {
    match action {
        ActionBacklight::Sample => probe::sample(),
        ActionBacklight::Set { percent } => {
            exec::ok("brightnessctl", &["set", &format!("{percent}%")]);
            Some(EventBacklight::Finished)
        }
    }
}
