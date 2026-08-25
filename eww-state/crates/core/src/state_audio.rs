//! Volume: the bar icon and the slider popup.

use crate::action::Action;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::model::Audio;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateAudio {
    level: Audio,
    /// Where the user has dragged the slider to, not yet asked for. A whole
    /// burst of drag events in one batch collapses to the value it ended on.
    wanted: Option<u8>,
    /// Number of `set-volume` calls dispatched but not yet finished.
    ///
    /// While any is outstanding the probe's answer describes the volume from
    /// *before* the drag, so it is ignored. This is what stopped the slider
    /// snapping backwards mid-drag: the poll and the setter no longer race.
    inflight: u32,
    /// The setters have all reported back, so it is worth asking what the
    /// volume actually ended up as.
    confirm: bool,
}

#[derive(Debug)]
pub enum EventAudio {
    /// The user moved the slider.
    Set {
        percent: u8,
    },
    Observed {
        audio: Audio,
    },
    Finished,
}

impl StateAudio {
    pub fn level(&self) -> Audio {
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

impl State for StateAudio {
    type Event = EventAudio;

    fn handle_event(&mut self, _cfg: &Config, event: EventAudio) {
        match event {
            EventAudio::Set { percent } => {
                let percent = percent.min(100);
                // Optimistic: the slider shows the requested value immediately
                // and the probe confirms it later.
                self.level.volume = percent;
                self.wanted = Some(percent);
            }
            EventAudio::Observed { audio } => {
                if self.inflight == 0 && self.wanted.is_none() {
                    self.level = audio;
                }
            }
            EventAudio::Finished => {
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
                ActionShell::SetVolume { percent },
            ));
        }
        if self.confirm && self.inflight == 0 {
            self.confirm = false;
            actions.push(Action::probe(jobs.take(), Probe::Audio));
        }
        None
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
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

    fn level(volume: u8, muted: bool) -> Audio {
        Audio { volume, muted }
    }

    #[test]
    fn drag_wins_over_a_stale_poll() {
        let mut a = StateAudio::default();
        run(&mut a, EventAudio::Set { percent: 70 });
        // The probe was already in flight with the old value when we set 70.
        run(
            &mut a,
            EventAudio::Observed {
                audio: level(20, false),
            },
        );
        assert_eq!(a.level.volume, 70, "a stale sample must not undo the drag");
        run(&mut a, EventAudio::Finished);
        run(
            &mut a,
            EventAudio::Observed {
                audio: level(70, false),
            },
        );
        assert_eq!(a.level.volume, 70);
    }

    #[test]
    fn settles_on_the_last_of_a_burst() {
        let mut a = StateAudio::default();
        for percent in [10, 20, 30] {
            run(&mut a, EventAudio::Set { percent });
        }
        run(&mut a, EventAudio::Finished);
        run(&mut a, EventAudio::Finished);
        // Two of three done: still ignoring samples.
        run(
            &mut a,
            EventAudio::Observed {
                audio: level(10, false),
            },
        );
        assert_eq!(a.level.volume, 30);
        run(&mut a, EventAudio::Finished);
        run(
            &mut a,
            EventAudio::Observed {
                audio: level(30, true),
            },
        );
        assert!(a.level.muted);
    }

    #[test]
    fn icon_tracks_level() {
        let cfg = Config::new("/i", "/c");
        let mut a = StateAudio::default();
        run(&mut a, EventAudio::Set { percent: 0 });
        assert!(a.icon(&cfg).ends_with("audio-volume-low-symbolic.svg"));
        run(&mut a, EventAudio::Set { percent: 80 });
        assert!(a.icon(&cfg).ends_with("audio-volume-high-symbolic.svg"));
    }
}
