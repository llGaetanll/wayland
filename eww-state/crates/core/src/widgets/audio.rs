//! Volume: the bar icon and the slider popup.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;
use crate::model::Audio as Level;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Audio {
    level: Level,
    /// Number of `set-volume` calls dispatched but not yet finished.
    ///
    /// While any is outstanding the probe's answer describes the volume from
    /// *before* the drag, so it is ignored. This is what stopped the slider
    /// snapping backwards mid-drag: the poll and the setter no longer race.
    inflight: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    /// The user moved the slider.
    Set(u8),
    Observed(Level),
    Done,
}

impl Audio {
    pub fn level(&self) -> Level {
        self.level
    }

    pub(crate) fn icon(&self, cfg: &Config) -> String {
        cfg.icon(if self.level.muted {
            "audio-volume-muted-symbolic"
        } else if self.level.volume <= 33 {
            "audio-volume-low-symbolic"
        } else if self.level.volume <= 66 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        })
    }
}

impl Widget for Audio {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::Set(v) => {
                let v = v.min(100);
                // Optimistic: the slider shows the requested value immediately
                // and the probe confirms it later.
                self.level.volume = v;
                self.inflight += 1;
                cx.dispatch(ActionKind::SetVolume(v));
            }
            Ev::Observed(level) => {
                if self.inflight == 0 {
                    self.level = level;
                }
            }
            Ev::Done => {
                self.inflight = self.inflight.saturating_sub(1);
                if self.inflight == 0 {
                    cx.dispatch(ActionKind::Probe(Probe::Audio));
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut Vars) {
        out.set("vol_pct", self.level.volume);
        out.set("vol_muted", self.level.muted);
        out.set("vol_icon", self.icon(cfg));
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Audio, cfg.poll.audio);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::run;

    #[test]
    fn drag_wins_over_a_stale_poll() {
        let mut a = Audio::default();
        run(&mut a, Ev::Set(70));
        // The probe was already in flight with the old value when we set 70.
        run(
            &mut a,
            Ev::Observed(Level {
                volume: 20,
                muted: false,
            }),
        );
        assert_eq!(a.level.volume, 70, "a stale sample must not undo the drag");
        run(&mut a, Ev::Done);
        run(
            &mut a,
            Ev::Observed(Level {
                volume: 70,
                muted: false,
            }),
        );
        assert_eq!(a.level.volume, 70);
    }

    #[test]
    fn settles_on_the_last_of_a_burst() {
        let mut a = Audio::default();
        for v in [10, 20, 30] {
            run(&mut a, Ev::Set(v));
        }
        run(&mut a, Ev::Done);
        run(&mut a, Ev::Done);
        // Two of three done: still ignoring samples.
        run(
            &mut a,
            Ev::Observed(Level {
                volume: 10,
                muted: false,
            }),
        );
        assert_eq!(a.level.volume, 30);
        run(&mut a, Ev::Done);
        run(
            &mut a,
            Ev::Observed(Level {
                volume: 30,
                muted: true,
            }),
        );
        assert!(a.level.muted);
    }

    #[test]
    fn icon_tracks_level() {
        let cfg = Config::new("/i", "/c");
        let mut a = Audio::default();
        run(&mut a, Ev::Set(0));
        assert!(a.icon(&cfg).ends_with("audio-volume-low-symbolic.svg"));
        run(&mut a, Ev::Set(80));
        assert!(a.icon(&cfg).ends_with("audio-volume-high-symbolic.svg"));
    }
}
