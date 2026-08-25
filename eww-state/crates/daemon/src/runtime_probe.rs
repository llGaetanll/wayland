//! Carrying out a probe action.

use eww_state_core::ActionProbe;
use eww_state_core::Event;

use crate::probe;
use crate::runtime_daemon::RuntimeDaemon;

impl RuntimeDaemon {
    pub(crate) fn perform_action_probe(&self, action: ActionProbe) -> (bool, Option<String>) {
        let ActionProbe::Sample { what } = action;
        match probe::sample(what, self.paths()) {
            Some(observed) => {
                self.send(Event::FromProbe(observed));
                (true, None)
            }
            None => {
                tracing::debug!(probe = what.as_str(), "nothing to sample");
                (false, None)
            }
        }
    }
}
