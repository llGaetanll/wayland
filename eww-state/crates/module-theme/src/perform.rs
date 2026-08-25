//! Carrying out what the theme module asked for.

use crate::action::ActionTheme;
use crate::event::EventTheme;
use crate::probe;

pub fn perform(action: &ActionTheme) -> Option<EventTheme> {
    match action {
        ActionTheme::Sample => Some(probe::sample()),
    }
}
