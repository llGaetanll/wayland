//! The brightness state machine. No IO.

use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionBacklight;
use crate::config::ConfigBacklight;
use crate::event::EventBacklight;

/// Never go below this: a 0% backlight is a black screen with no way back.
const MIN: u8 = 1;

#[derive(Clone, Debug)]
pub struct StateBacklight {
    cfg: ConfigBacklight,
    percent: u8,
    /// See the note in `module-audio` — the same stale-sample guard, and the
    /// same collapse of a drag's worth of events into one command.
    wanted: Option<u8>,
    inflight: u32,
    want_sample: bool,
}

impl Default for StateBacklight {
    fn default() -> Self {
        StateBacklight {
            cfg: ConfigBacklight::default(),
            percent: MIN,
            wanted: None,
            inflight: 0,
            want_sample: false,
        }
    }
}

impl StateBacklight {
    pub fn new(cfg: ConfigBacklight) -> StateBacklight {
        StateBacklight {
            cfg,
            ..StateBacklight::default()
        }
    }

    pub fn percent(&self) -> u8 {
        self.percent
    }

    pub fn icon(&self) -> String {
        self.cfg.icon(if self.percent <= 1 {
            "display-brightness-off-symbolic"
        } else if self.percent <= 33 {
            "display-brightness-low-symbolic"
        } else if self.percent <= 66 {
            "display-brightness-medium-symbolic"
        } else {
            "display-brightness-symbolic"
        })
    }
}

impl Module for StateBacklight {
    type Event = EventBacklight;
    type Action = ActionBacklight;
    type Req = ();

    fn handle_event(&mut self, event: &EventBacklight) {
        match event {
            EventBacklight::Due => self.want_sample = true,
            EventBacklight::Set { percent } => {
                self.percent = (*percent).clamp(MIN, 100);
                self.wanted = Some(self.percent);
            }
            EventBacklight::Observed { percent } => {
                if self.inflight == 0 && self.wanted.is_none() {
                    self.percent = (*percent).clamp(MIN, 100);
                }
            }
            EventBacklight::Finished => {
                self.inflight = self.inflight.saturating_sub(1);
                self.want_sample = self.want_sample || self.inflight == 0;
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionBacklight>) -> Option<()> {
        if let Some(percent) = self.wanted.take() {
            self.inflight += 1;
            self.want_sample = false;
            actions.push(ActionBacklight::Set { percent });
        }
        if std::mem::take(&mut self.want_sample) && self.inflight == 0 {
            actions.push(ActionBacklight::Sample);
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        out.set("bri_pct", self.percent);
        out.set("bri_icon", self.icon());
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
