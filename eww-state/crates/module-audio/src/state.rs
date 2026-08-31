//! The volume state machine. No IO.

use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionAudio;
use crate::config::ConfigAudio;
use crate::event::EventAudio;
use crate::model::Audio;

#[derive(Clone, Debug, Default)]
pub struct StateAudio {
    cfg: ConfigAudio,
    level: Audio,
    /// Where the user has dragged the slider to, not yet asked for. A whole
    /// burst of drag events in one batch collapses to the value it ended on.
    wanted: Option<u8>,
    /// Number of `set-volume` calls dispatched but not yet finished.
    ///
    /// While any is outstanding a sample describes the volume from *before* the
    /// drag, so it is ignored. This is what stopped the slider snapping
    /// backwards mid-drag: the poll and the setter no longer race.
    inflight: u32,
    want_sample: bool,
}

impl StateAudio {
    /// Follow the desktop's light/dark preference. The parent owns the theme
    /// and hands it down; this module never asks what it is.
    pub fn set_icons_dir(&mut self, dir: &str) {
        self.cfg.set_icons_dir(dir);
    }

    pub fn new(cfg: ConfigAudio) -> StateAudio {
        StateAudio {
            cfg,
            ..StateAudio::default()
        }
    }

    pub fn level(&self) -> Audio {
        self.level
    }

    pub fn icon(&self) -> String {
        self.cfg.icon(if self.level.muted {
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

impl Module for StateAudio {
    type Event = EventAudio;
    type Action = ActionAudio;
    type Req = ();

    fn handle_event(&mut self, event: &EventAudio) {
        match event {
            EventAudio::Due => self.want_sample = true,
            EventAudio::Set { percent } => {
                // Optimistic: the slider shows the requested value immediately
                // and a sample confirms it later.
                self.level.volume = (*percent).min(100);
                self.wanted = Some(self.level.volume);
            }
            EventAudio::Observed { audio } => {
                if self.inflight == 0 && self.wanted.is_none() {
                    self.level = *audio;
                }
            }
            EventAudio::Finished => {
                self.inflight = self.inflight.saturating_sub(1);
                self.want_sample = self.want_sample || self.inflight == 0;
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionAudio>) -> Option<()> {
        if let Some(percent) = self.wanted.take() {
            self.inflight += 1;
            self.want_sample = false;
            actions.push(ActionAudio::Set { percent });
        }
        if std::mem::take(&mut self.want_sample) && self.inflight == 0 {
            actions.push(ActionAudio::Sample);
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        out.set("vol_pct", self.level.volume);
        out.set("vol_muted", self.level.muted);
        out.set("vol_icon", self.icon());
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
