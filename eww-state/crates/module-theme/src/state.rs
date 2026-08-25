//! The theme state machine. No IO.

use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionTheme;
use crate::config::ConfigTheme;
use crate::event::EventTheme;
use crate::model::Theme;

#[derive(Clone, Debug, Default)]
pub struct StateTheme {
    cfg: ConfigTheme,
    mode: Theme,
    want_sample: bool,
}

impl StateTheme {
    pub fn new(cfg: ConfigTheme) -> StateTheme {
        StateTheme {
            cfg,
            ..StateTheme::default()
        }
    }

    pub fn mode(&self) -> Theme {
        self.mode
    }
}

impl Module for StateTheme {
    type Event = EventTheme;
    type Action = ActionTheme;
    type Req = ();

    fn handle_event(&mut self, event: &EventTheme) {
        match event {
            EventTheme::Due => self.want_sample = true,
            EventTheme::Observed { theme } => self.mode = *theme,
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionTheme>) -> Option<()> {
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionTheme::Sample);
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        out.set("theme", self.mode.as_str());
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
