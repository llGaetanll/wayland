//! Screen brightness: the bar icon and the slider popup.

use crate::action::Action;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

/// Never go below this: a 0% backlight is a black screen with no way back.
const MIN: u8 = 1;

#[derive(Clone, Debug)]
pub struct StateBacklight {
    percent: u8,
    /// See the note in [`crate::state_audio`] — same stale-sample guard, and
    /// the same collapse of a drag's worth of events into one command.
    wanted: Option<u8>,
    inflight: u32,
    confirm: bool,
}

impl Default for StateBacklight {
    fn default() -> Self {
        StateBacklight {
            percent: MIN,
            wanted: None,
            inflight: 0,
            confirm: false,
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

    fn handle_event(&mut self, _cfg: &Config, event: EventBacklight) {
        match event {
            EventBacklight::Set { percent } => {
                let percent = percent.clamp(MIN, 100);
                self.percent = percent;
                self.wanted = Some(percent);
            }
            EventBacklight::Observed { percent } => {
                if self.inflight == 0 && self.wanted.is_none() {
                    self.percent = percent.clamp(MIN, 100);
                }
            }
            EventBacklight::Finished => {
                self.inflight = self.inflight.saturating_sub(1);
                self.confirm = self.inflight == 0;
            }
        }
    }

    fn emit_actions(
        &mut self,
        _cfg: &Config,
        jobs: &mut Jobs,
        actions: &mut Vec<Action>,
    ) -> Option<Req> {
        if let Some(percent) = self.wanted.take() {
            self.inflight += 1;
            self.confirm = false;
            actions.push(Action::shell(
                jobs.take(),
                ActionShell::SetBrightness { percent },
            ));
        }
        if self.confirm && self.inflight == 0 {
            self.confirm = false;
            actions.push(Action::probe(jobs.take(), Probe::Backlight));
        }
        None
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
