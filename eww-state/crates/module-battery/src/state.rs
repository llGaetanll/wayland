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

    /// Follow the desktop's light/dark preference. Redrawing is forced by
    /// forgetting what is on disk: the path encodes the appearance, so the new
    /// glyph lands at a new path and GTK, which caches by path, reloads it.
    ///
    /// `drawing` is dropped too, and that is the whole point of it. The
    /// appearance is not part of what a render reports back, so a render that
    /// was already in flight for the *old* appearance would otherwise be
    /// accepted as having drawn the new one: `drawn` would be set, `icon()`
    /// would name the new path, and the file at that path would never have been
    /// written — the pill vanishes from the bar for the rest of the session.
    /// Forgetting the job id makes its result unmatchable, so it is ignored.
    pub fn set_dark(&mut self, dark: bool) {
        if self.cfg.dark == dark {
            return;
        }
        self.cfg.dark = dark;
        self.drawn = None;
        self.failed = None;
        self.drawing = None;
    }

    pub fn level(&self) -> Option<Battery> {
        self.level
    }

    /// The pill's path, or nothing while there is no pill to point at.
    pub fn icon(&self) -> String {
        self.drawn
            .map(|drawn| svg::path(&self.cfg.cache_dir, drawn, self.cfg.dark))
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
            actions.push(ActionBattery::Render {
                id,
                battery: level,
                dark: self.cfg.dark,
            });
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

#[cfg(test)]
mod tests {
    use api::module::drive;

    use super::*;

    fn level(percent: u8) -> Battery {
        Battery {
            percent,
            charging: false,
        }
    }

    /// Switching appearance mid-render must not let the old render count as the
    /// new one. It drew the other file, and the path we publish encodes which.
    #[test]
    fn a_render_in_flight_does_not_survive_a_theme_change() {
        let mut b = StateBattery::new(ConfigBattery {
            cache_dir: "/c".to_string(),
            dark: false,
            ..ConfigBattery::default()
        });
        b.handle_event(&EventBattery::Observed { battery: level(50) });
        let mut actions = Vec::new();
        b.emit_actions(&mut actions);
        let id = match actions.first() {
            Some(ActionBattery::Render { id, .. }) => *id,
            other => panic!("expected a render, got {other:?}"),
        };

        b.set_dark(true);
        b.handle_event(&EventBattery::Finished { id, ok: true });
        assert_eq!(
            b.icon(),
            "",
            "the light render must not be published as the dark pill"
        );

        // ...and the module asks for the dark one instead.
        let (actions, _) = drive(&mut b, EventBattery::Observed { battery: level(50) });
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, ActionBattery::Render { dark: true, .. })),
            "expected a redraw for the new appearance, got {actions:?}"
        );
    }
}
