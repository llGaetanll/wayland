//! The battery pill.
//!
//! The pill is an SVG the runtime regenerates whenever the drawing would
//! change. Its filename encodes capacity and charge state, so the path changes
//! exactly when the picture does — which is what makes eww and GTK reload it,
//! since new bytes at a cached path would not.

use crate::action::Action;
use crate::action_file::ActionFile;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::model::Battery;
use crate::probe::JobId;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateBattery {
    /// `None` until the first sample. There is no such thing as a battery
    /// reading we have not taken, and a default one is a fabricated 0%.
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
    /// The picture whose render just failed.
    ///
    /// Without this the retry is immediate, and a failure that reports back
    /// instantly spins: emit, fail, emit. A changed reading clears it, which is
    /// the next sample away at worst.
    failed: Option<Battery>,
}

#[derive(Debug)]
pub enum EventBattery {
    Observed { battery: Battery },
    Finished { id: JobId, ok: bool },
}

impl StateBattery {
    pub fn level(&self) -> Option<Battery> {
        self.level
    }

    fn icon(&self, cfg: &Config) -> String {
        let Some(drawn) = self.drawn else {
            return String::new();
        };
        format!(
            "{}/battery-{}-{}.svg",
            cfg.cache_dir,
            drawn.percent,
            u8::from(drawn.charging)
        )
    }
}

impl State for StateBattery {
    type Event = EventBattery;

    fn handle_event(&mut self, _cfg: &Config, event: EventBattery) {
        match event {
            EventBattery::Observed { battery } => {
                self.level = Some(Battery {
                    percent: battery.percent.min(100),
                    ..battery
                });
            }
            EventBattery::Finished { id, ok } => {
                let Some((pending, battery)) = self.drawing else {
                    return;
                };
                if pending != id {
                    return;
                }
                self.drawing = None;
                // A failed render leaves `drawn` alone, so the bar is never
                // pointed at a file nobody wrote.
                if ok {
                    self.drawn = Some(battery);
                    self.failed = None;
                } else {
                    self.failed = Some(battery);
                }
            }
        }
    }

    fn emit_actions(
        &mut self,
        _cfg: &Config,
        jobs: &mut Jobs,
        actions: &mut Vec<Action>,
    ) -> Option<Req> {
        let Some(level) = self.level else {
            return None; // nothing sampled yet, so nothing to draw
        };
        let drawing = self.drawing.map(|(_, battery)| battery);
        if self.drawn != Some(level) && drawing != Some(level) && self.failed != Some(level) {
            let id = jobs.take();
            actions.push(Action::file(
                id,
                ActionFile::RenderBattery {
                    percent: level.percent,
                    charging: level.charging,
                },
            ));
            self.drawing = Some((id, level));
        }
        None
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
        let level = self.level.unwrap_or_default();
        out.set("batt_icon", self.icon(cfg));
        out.set("batt_pct", level.percent);
        out.set("batt_charging", level.charging);
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Battery, cfg.poll.battery);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::run;

    fn level(percent: u8, charging: bool) -> Battery {
        Battery { percent, charging }
    }

    fn observed(percent: u8, charging: bool) -> EventBattery {
        EventBattery::Observed {
            battery: level(percent, charging),
        }
    }

    fn icon(b: &StateBattery) -> String {
        let mut vars = WireVars::default();
        b.vars(&Config::new("/i", "/c"), &mut vars);
        vars.get("batt_icon").unwrap_or_default().to_string()
    }

    #[test]
    fn redraws_only_when_the_picture_changes() {
        let mut b = StateBattery::default();
        let actions = run(&mut b, observed(55, false));
        assert_eq!(actions.len(), 1);
        run(
            &mut b,
            EventBattery::Finished {
                id: actions[0].id(),
                ok: true,
            },
        );
        assert!(run(&mut b, observed(55, false)).is_empty());
        assert_eq!(
            run(&mut b, observed(55, true)).len(),
            1,
            "plugging in changes the fill colour"
        );
    }

    #[test]
    fn the_path_names_a_file_that_exists() {
        let mut b = StateBattery::default();
        let actions = run(&mut b, observed(55, false));
        assert_eq!(
            icon(&b),
            "",
            "publishing a path before the file is written poisons GTK's cache"
        );
        run(
            &mut b,
            EventBattery::Finished {
                id: actions[0].id(),
                ok: true,
            },
        );
        assert_eq!(icon(&b), "/c/battery-55-0.svg");

        // The next sample's pill is not on disk yet, so the bar keeps showing
        // the one that is.
        run(&mut b, observed(54, false));
        assert_eq!(icon(&b), "/c/battery-55-0.svg");
    }

    #[test]
    fn a_failed_render_does_not_spin() {
        let mut b = StateBattery::default();
        let actions = run(&mut b, observed(30, false));
        let failed = run(
            &mut b,
            EventBattery::Finished {
                id: actions[0].id(),
                ok: false,
            },
        );
        assert!(
            failed.is_empty(),
            "retrying the moment it failed is a hot loop, not a retry"
        );
        assert_eq!(icon(&b), "");
        assert!(run(&mut b, observed(30, false)).is_empty());

        // A different picture is a different file, so it is worth trying.
        assert_eq!(run(&mut b, observed(31, false)).len(), 1);
    }

    #[test]
    fn a_late_render_does_not_confirm_the_wrong_picture() {
        let mut b = StateBattery::default();
        let stale = run(&mut b, observed(80, false));
        // Charge state flips before the first render reports back.
        let fresh = run(&mut b, observed(80, true));
        run(
            &mut b,
            EventBattery::Finished {
                id: stale[0].id(),
                ok: true,
            },
        );
        assert_eq!(icon(&b), "", "the stale render confirms nothing");
        run(
            &mut b,
            EventBattery::Finished {
                id: fresh[0].id(),
                ok: true,
            },
        );
        assert_eq!(icon(&b), "/c/battery-80-1.svg");
    }
}
