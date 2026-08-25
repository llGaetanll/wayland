//! The bar clock.
//!
//! Formatting is the shell's job — a state machine with no IO has no clock, so
//! the already-formatted string arrives as an observation.

use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Clock {
    text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Observed(String),
}

impl Widget for Clock {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, _cx: &mut Ctx) {
        let Ev::Observed(text) = ev;
        self.text = text;
    }

    fn vars(&self, _cfg: &Config, out: &mut Vars) {
        out.set("clock", &self.text);
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Clock, cfg.poll.clock);
    }
}
