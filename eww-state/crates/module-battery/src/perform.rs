//! Carrying out what the battery module asked for.

use std::path::Path;

use crate::action::ActionBattery;
use crate::event::EventBattery;
use crate::probe;
use crate::svg;

pub fn perform(action: &ActionBattery, cache: &Path) -> Option<EventBattery> {
    match action {
        ActionBattery::Sample => Some(probe::sample()),
        ActionBattery::Render { id, battery } => Some(EventBattery::Finished {
            id: *id,
            ok: svg::render(cache, *battery).is_ok(),
        }),
    }
}
