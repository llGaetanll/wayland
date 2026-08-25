//! The battery state machine. No IO.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionBattery;
use crate::config::ConfigBattery;
use crate::event::EventBattery;
use crate::model::Battery;
use crate::svg;

#[derive(Clone, Debug, Default)]
pub struct StateBattery {
    cfg: ConfigBattery,
    jobs: Jobs,
    /// `None` until the first sample. There is no such thing as a reading we
    /// have not taken, and a default one is a fabricated 0%.
    level: Option<Battery>,
    /// The pill the runtime has confirmed is on disk.
    ///
    /// Nothing else may be published as the icon path. GTK caches a load that
    /// failed, and the reconciler sends a var only when it changes — so naming
    /// a file that does not exist yet costs the pill for the rest of the
    /// session, not just until the next sample.
    drawn: Option<Battery>,
    /// The render we are waiting on, and what it is drawing.
    drawing: Option<(JobId, Battery)>,
    /// The picture whose render just failed. Without this the retry is
    /// immediate, and a failure that reports back instantly spins.
    failed: Option<Battery>,
    want_sample: bool,
}

impl StateBattery {
    pub fn new(cfg: ConfigBattery) -> StateBattery {
        StateBattery {
            cfg,
            ..StateBattery::default()
        }
    }

    pub fn level(&self) -> Option<Battery> {
        self.level
    }

    /// The pill's path, or nothing while there is no pill to point at.
    pub fn icon(&self) -> String {
        self.drawn
            .map(|drawn| svg::path(&self.cfg.cache_dir, drawn))
            .unwrap_or_default()
    }
}

impl Module for StateBattery {
    type Event = EventBattery;
    type Action = ActionBattery;
    type Req = ();

    fn handle_event(&mut self, event: &EventBattery) {
        match event {
            EventBattery::Due => self.want_sample = true,
            EventBattery::Observed { battery } => {
                self.level = Some(Battery {
                    percent: battery.percent.min(100),
                    ..*battery
                });
            }
            EventBattery::Finished { id, ok } => {
                let Some((pending, battery)) = self.drawing else {
                    return;
                };
                if pending != *id {
                    return; // a render for a picture that has since changed
                }
                self.drawing = None;
                if *ok {
                    self.drawn = Some(battery);
                    self.failed = None;
                } else {
                    self.failed = Some(battery);
                }
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionBattery>) -> Option<()> {
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionBattery::Sample);
        }
        let Some(level) = self.level else {
            return None; // nothing sampled yet, so nothing to draw
        };
        let drawing = self.drawing.map(|(_, battery)| battery);
        if self.drawn != Some(level) && drawing != Some(level) && self.failed != Some(level) {
            let id = self.jobs.take();
            actions.push(ActionBattery::Render { id, battery: level });
            self.drawing = Some((id, level));
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let level = self.level.unwrap_or_default();
        out.set("batt_icon", self.icon());
        out.set("batt_pct", level.percent);
        out.set("batt_charging", level.charging);
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
