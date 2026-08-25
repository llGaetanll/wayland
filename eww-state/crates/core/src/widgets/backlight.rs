//! Screen brightness: the bar icon and the slider popup.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;
use crate::widgets::{PollPlan, Widget};

/// Never go below this: a 0% backlight is a black screen with no way back.
const MIN: u8 = 1;

#[derive(Clone, Debug)]
pub struct Backlight {
    percent: u8,
    /// See the note in [`crate::widgets::audio`] — same stale-sample guard.
    inflight: u32,
}

impl Default for Backlight {
    fn default() -> Self {
        Backlight {
            percent: MIN,
            inflight: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Set(u8),
    Observed(u8),
    Done,
}

impl Backlight {
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

impl Widget for Backlight {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::Set(v) => {
                let v = v.clamp(MIN, 100);
                self.percent = v;
                self.inflight += 1;
                cx.dispatch(ActionKind::SetBrightness(v));
            }
            Ev::Observed(v) => {
                if self.inflight == 0 {
                    self.percent = v.clamp(MIN, 100);
                }
            }
            Ev::Done => {
                self.inflight = self.inflight.saturating_sub(1);
                if self.inflight == 0 {
                    cx.dispatch(ActionKind::Probe(Probe::Backlight));
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut Vars) {
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
        let mut b = Backlight::default();
        run(&mut b, Ev::Set(0));
        assert_eq!(b.percent, MIN);
        run(&mut b, Ev::Observed(0));
        assert_eq!(b.percent, MIN, "even a probe cannot report us to black");
    }

    #[test]
    fn stale_sample_does_not_undo_a_drag() {
        let mut b = Backlight::default();
        run(&mut b, Ev::Set(90));
        run(&mut b, Ev::Observed(10));
        assert_eq!(b.percent, 90);
    }
}
