//! Screen brightness: the bar icon and the slider popup.

use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::ctx::Ctx;
use crate::probe::Probe;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

/// Never go below this: a 0% backlight is a black screen with no way back.
const MIN: u8 = 1;

#[derive(Clone, Debug)]
pub struct StateBacklight {
    percent: u8,
    /// See the note in [`crate::state_audio`] — same stale-sample guard.
    inflight: u32,
}

impl Default for StateBacklight {
    fn default() -> Self {
        StateBacklight {
            percent: MIN,
            inflight: 0,
        }
    }
}

#[derive(Debug)]
pub enum EventBacklight {
    Set { percent: u8 },
    Observed { percent: u8 },
    Finished,
}

impl StateBacklight {
    pub fn percent(&self) -> u8 {
        self.percent
    }

    pub(crate) fn icon(&self, cfg: &Config) -> String {
        cfg.icon(if self.percent <= 1 {
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

impl State for StateBacklight {
    type Event = EventBacklight;

    fn handle_event(&mut self, _cfg: &Config, event: EventBacklight, cx: &mut Ctx) {
        match event {
            EventBacklight::Set { percent } => {
                let percent = percent.clamp(MIN, 100);
                self.percent = percent;
                self.inflight += 1;
                cx.shell(ActionShell::SetBrightness { percent });
            }
            EventBacklight::Observed { percent } => {
                if self.inflight == 0 {
                    self.percent = percent.clamp(MIN, 100);
                }
            }
            EventBacklight::Finished => {
                self.inflight = self.inflight.saturating_sub(1);
                if self.inflight == 0 {
                    cx.probe(Probe::Backlight);
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
        out.set("bri_pct", self.percent);
        out.set("bri_icon", self.icon(cfg));
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Backlight, cfg.poll.backlight);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::run;

    #[test]
    fn never_reaches_zero() {
        let mut b = StateBacklight::default();
        run(&mut b, EventBacklight::Set { percent: 0 });
        assert_eq!(b.percent, MIN);
        run(&mut b, EventBacklight::Observed { percent: 0 });
        assert_eq!(b.percent, MIN, "even a probe cannot report us to black");
    }

    #[test]
    fn stale_sample_does_not_undo_a_drag() {
        let mut b = StateBacklight::default();
        run(&mut b, EventBacklight::Set { percent: 90 });
        run(&mut b, EventBacklight::Observed { percent: 10 });
        assert_eq!(b.percent, 90);
    }
}
