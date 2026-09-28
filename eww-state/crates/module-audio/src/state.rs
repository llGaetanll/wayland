//! The volume state machine. No IO.

use api::module::Module;
use api::util::b64;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionAudio;
use crate::config::ConfigAudio;
use crate::event::EventAudio;
use crate::model::Audio;
use crate::model::AudioOutputs;

#[derive(Clone, Debug, Default)]
pub struct StateAudio {
    cfg: ConfigAudio,
    level: Audio,
    outputs: AudioOutputs,
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
    /// The sink a click asked for, not yet sent.
    wanted_output: Option<String>,
    /// A switch is out and unanswered. A listing taken while it settles still
    /// names the old default, which would move the tick back under the click.
    switching: bool,
    want_outputs: bool,
    menu_open: bool,
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

    pub fn outputs(&self) -> &AudioOutputs {
        &self.outputs
    }

    /// A switch is out and has not been answered.
    pub fn settling(&self) -> bool {
        self.switching
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
            EventAudio::Due => {
                self.want_sample = true;
                // Only while the list is on screen. Off screen nothing shows
                // the outputs, and this is the expensive half of the module.
                self.want_outputs = self.want_outputs || self.menu_open;
            }
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
            EventAudio::MenuOpened => {
                self.menu_open = true;
                self.want_sample = true;
                self.want_outputs = true;
            }
            EventAudio::MenuClosed => self.menu_open = false,
            EventAudio::OutputPicked { id } => {
                // Switching to the sink already in use is not a command, it is
                // a click on the row with the tick already against it.
                if !self.switching && self.wanted_output.is_none() && *id != self.outputs.default {
                    self.wanted_output = Some(id.clone());
                }
            }
            EventAudio::OutputsObserved { outputs } => {
                if !self.switching && self.wanted_output.is_none() {
                    self.outputs = outputs.clone();
                }
            }
            EventAudio::OutputSwitched { ok } => {
                self.switching = false;
                self.want_outputs = true;
                // Volume is per sink, so the slider is describing the old one
                // until this lands.
                self.want_sample = *ok || self.want_sample;
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionAudio>) -> Option<()> {
        if let Some(percent) = self.wanted.take() {
            self.inflight += 1;
            self.want_sample = false;
            actions.push(ActionAudio::Set { percent });
        }
        if let Some(id) = self.wanted_output.take() {
            // Optimistic, like the slider: the tick moves to the row that was
            // clicked and the next listing confirms it.
            self.outputs.default = id.clone();
            self.switching = true;
            actions.push(ActionAudio::SetDefaultOutput { id });
        }
        if std::mem::take(&mut self.want_outputs) && !self.switching {
            actions.push(ActionAudio::SampleOutputs);
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

        let sinks: Vec<WireJson> = self
            .outputs
            .sinks
            .iter()
            .map(|s| {
                let active = s.id == self.outputs.default;
                WireJson::Obj(vec![
                    ("id", WireJson::s(&s.id)),
                    // Sink names come off the hardware and reach the handler as
                    // part of a shell command, so they travel encoded like an
                    // SSID does.
                    ("id_b64", WireJson::s(b64(&s.id))),
                    ("name", WireJson::s(&s.name)),
                    ("active", WireJson::Bool(active)),
                    ("icon", WireJson::s(self.cfg.output_icon(s.kind, active))),
                ])
            })
            .collect();
        out.set("vol_outputs", WireJson::Arr(sinks));
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
