//! Light/dark preference, published as a CSS class every surface picks up.

use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;
use crate::model::Theme as Mode;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Theme {
    mode: Mode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Observed(Mode),
}

impl Theme {
    pub fn mode(&self) -> Mode {
        self.mode
    }
}

impl Widget for Theme {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, _cx: &mut Ctx) {
        let Ev::Observed(mode) = ev;
        self.mode = mode;
    }

    fn vars(&self, _cfg: &Config, out: &mut Vars) {
        out.set("theme", self.mode.as_str());
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Theme, cfg.poll.theme);
    }
}
