//! The date state machine. No IO.

use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionDate;
use crate::config::ConfigDate;
use crate::event::EventDate;

#[derive(Clone, Debug, Default)]
pub struct StateDate {
    cfg: ConfigDate,
    text: String,
    want_sample: bool,
}

impl StateDate {
    pub fn new(cfg: ConfigDate) -> StateDate {
        StateDate {
            cfg,
            ..StateDate::default()
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn format(&self) -> &str {
        &self.cfg.format
    }
}

impl Module for StateDate {
    type Event = EventDate;
    type Action = ActionDate;
    type Req = ();

    fn handle_event(&mut self, event: &EventDate) {
        match event {
            EventDate::Due => self.want_sample = true,
            EventDate::Observed { text } => self.text = text.clone(),
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionDate>) -> Option<()> {
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionDate::Sample);
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        out.set("clock", &self.text);
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
