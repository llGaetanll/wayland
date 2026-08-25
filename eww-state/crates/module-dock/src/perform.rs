//! Carrying out what the dock module asked for.

use std::path::Path;

use crate::action::ActionDock;
use crate::event::EventDock;
use crate::probe;

pub fn perform(action: &ActionDock, conf: &Path) -> Option<EventDock> {
    match action {
        ActionDock::Sample => Some(probe::sample(conf)),
        ActionDock::Launch { cmd } => {
            exec::detach(&format!("cd ~ && {cmd}"));
            None
        }
        ActionDock::Write { apps } => {
            if let Err(e) = probe::write(conf, apps) {
                tracing::error!(error = %e, "could not write the dock order");
            }
            None
        }
    }
}
