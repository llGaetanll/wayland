//! Carrying out what the battery module asked for.

use std::path::Path;

use crate::action::ActionBattery;
use crate::event::EventBattery;
use crate::graph;
use crate::probe;
use crate::svg;

pub fn perform(action: &ActionBattery, cache: &Path) -> Option<EventBattery> {
    match action {
        ActionBattery::Sample {
            history,
            span,
            every,
        } => Some(probe::sample(history, *span, *every)),
        ActionBattery::SampleProcs {
            window_ms,
            top,
            icons,
            owners,
            fallback_icon,
        } => Some(EventBattery::ProcsObserved {
            procs: probe::procs(*window_ms, *top, icons, owners, fallback_icon),
        }),
        ActionBattery::Render { id, battery, dark } => Some(EventBattery::Finished {
            id: *id,
            ok: svg::render(cache, *battery, *dark).is_ok(),
        }),
        ActionBattery::RenderGraph {
            id,
            points,
            span,
            dark,
        } => Some(EventBattery::GraphFinished {
            id: *id,
            ok: graph::render(cache, points, *span, *dark).is_ok(),
        }),
    }
}
