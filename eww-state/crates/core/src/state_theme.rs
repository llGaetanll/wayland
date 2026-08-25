//! Light/dark preference, published as a CSS class every surface picks up.

use crate::config::Config;
use crate::model::Theme;
use crate::probe::Probe;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateTheme {
    mode: Theme,
}

#[derive(Debug)]
pub enum EventTheme {
    Observed { theme: Theme },
}

impl StateTheme {
    pub fn mode(&self) -> Theme {
        self.mode
    }
}

impl State for StateTheme {
    type Event = EventTheme;

    fn handle_event(&mut self, _cfg: &Config, event: EventTheme) {
        let EventTheme::Observed { theme } = event;
        self.mode = theme;
    }

    fn vars(&self, _cfg: &Config, out: &mut WireVars) {
        out.set("theme", self.mode.as_str());
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Theme, cfg.poll.theme);
    }
}
