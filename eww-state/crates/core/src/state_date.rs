//! The date and time shown in the bar.
//!
//! Formatting is the runtime's job — a state machine with no IO has no clock,
//! and the tick that does reach it carries an `Instant`, not a calendar. So the
//! already-formatted string arrives as an observation like any other.

use crate::config::Config;
use crate::ctx::Ctx;
use crate::probe::Probe;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateDate {
    text: String,
}

#[derive(Debug)]
pub enum EventDate {
    Observed { text: String },
}

impl State for StateDate {
    type Event = EventDate;

    fn handle_event(&mut self, _cfg: &Config, event: EventDate, _cx: &mut Ctx) {
        let EventDate::Observed { text } = event;
        self.text = text;
    }

    fn vars(&self, _cfg: &Config, out: &mut WireVars) {
        out.set("clock", &self.text);
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Date, cfg.poll.date);
    }
}
