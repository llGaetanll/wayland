//! The battery state machine. No IO.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::module::Module;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionBattery;
use crate::config::ConfigBattery;
use crate::event::EventBattery;
use crate::graph;
use crate::model::Battery;
use crate::model::HistPoint;
use crate::model::Proc;
use crate::model::Reading;
use crate::svg;
use crate::wire_fmt;

#[derive(Clone, Debug, Default)]
pub struct StateBattery {
    cfg: ConfigBattery,
    jobs: Jobs,
    /// `None` until the first sample. There is no such thing as a reading we
    /// have not taken, and a default one is a fabricated 0%.
    reading: Option<Reading>,
    history: Vec<HistPoint>,
    procs: Vec<Proc>,
    /// Dock ids and their icons, handed down by the parent. This module does
    /// not know there is a dock; it knows some names have pictures.
    icons: Vec<(String, String)>,
    /// Which process owns which window, handed down the same way.
    owners: Vec<(u32, String)>,
    menu_open: bool,
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
    /// The same three, for the chart. Keyed by fingerprint rather than by the
    /// points themselves: the ring is hundreds of points and this is compared
    /// on every emit.
    graph_drawn: Option<u64>,
    graph_drawing: Option<(JobId, u64)>,
    graph_failed: Option<u64>,
    want_sample: bool,
    want_procs: bool,
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
        self.graph_drawn = None;
        self.graph_failed = None;
        self.graph_drawing = None;
    }

    /// Let a row be drawn with the app's own picture. The dock's list is the
    /// dock's own; this is a copy handed down by the parent, which is the only
    /// direction anything here is allowed to speak in.
    pub fn set_app_icons(&mut self, icons: Vec<(String, String)>) {
        self.icons = icons;
    }

    /// Let CPU time be attributed to the app whose window owns it. The
    /// compositor's answer, handed down by the parent — this module never asks
    /// what a window is.
    pub fn set_window_owners(&mut self, owners: Vec<(u32, String)>) {
        self.owners = owners;
    }

    pub fn level(&self) -> Option<Battery> {
        self.reading.as_ref().map(|r| r.battery)
    }

    /// The pill's path, or nothing while there is no pill to point at.
    pub fn icon(&self) -> String {
        self.drawn
            .map(|drawn| svg::path(&self.cfg.cache_dir, drawn, self.cfg.dark))
            .unwrap_or_default()
    }

    /// The chart's path, on the same rule as the pill's.
    pub fn graph(&self) -> String {
        self.graph_drawn
            .map(|key| graph::path(&self.cfg.cache_dir, key))
            .unwrap_or_default()
    }

    /// What the panel says next to the estimate. Charging and discharging are
    /// different questions, and one label for both would be wrong half the time.
    fn detail(&self) -> WireJson {
        let reading = self.reading.clone().unwrap_or_default();
        let charging = reading.battery.charging;
        WireJson::Obj(vec![
            ("percent", WireJson::Int(reading.battery.percent.into())),
            ("charging", WireJson::Bool(charging)),
            (
                "source",
                WireJson::s(if charging { "Power Adapter" } else { "Battery" }),
            ),
            (
                "time_label",
                WireJson::s(if charging {
                    "Time until full"
                } else {
                    "Time remaining"
                }),
            ),
            (
                "time",
                WireJson::s(match reading.seconds_left {
                    Some(secs) => wire_fmt::hm(secs.into()),
                    // The kernel reports no current on a pack that is neither
                    // charging nor discharging, and there is no estimate to
                    // make. Saying so beats a zero that reads as "any moment".
                    None => "—".to_string(),
                }),
            ),
            ("watts", WireJson::s(wire_fmt::watts(reading.milliwatts))),
            ("health", WireJson::s(&reading.health)),
            ("cycles", WireJson::Int(reading.cycles.into())),
        ])
    }
}

impl Module for StateBattery {
    type Event = EventBattery;
    type Action = ActionBattery;
    type Req = ();

    fn handle_event(&mut self, event: &EventBattery) {
        match event {
            EventBattery::Due => {
                self.want_sample = true;
                // Only while the panel is on screen. Nothing shows the list
                // otherwise, and timing every process on the machine is by far
                // the most expensive thing this module does.
                self.want_procs = self.want_procs || self.menu_open;
            }
            EventBattery::Observed { reading, history } => {
                self.reading = Some(reading.clone());
                self.history = history.clone();
            }
            EventBattery::MenuOpened => {
                self.menu_open = true;
                self.want_sample = true;
                self.want_procs = true;
            }
            EventBattery::MenuClosed => self.menu_open = false,
            EventBattery::ProcsObserved { procs } => self.procs = procs.clone(),
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
            EventBattery::GraphFinished { id, ok } => {
                let Some((pending, key)) = self.graph_drawing else {
                    return;
                };
                if pending != *id {
                    return;
                }
                self.graph_drawing = None;
                if *ok {
                    self.graph_drawn = Some(key);
                    self.graph_failed = None;
                } else {
                    self.graph_failed = Some(key);
                }
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionBattery>) -> Option<()> {
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionBattery::Sample {
                history: self.cfg.history.clone(),
                span: self.cfg.span,
                every: self.cfg.every,
            });
        }
        if std::mem::take(&mut self.want_procs) {
            actions.push(ActionBattery::SampleProcs {
                window_ms: self.cfg.proc_window_ms,
                top: self.cfg.top,
                icons: self.icons.clone(),
                owners: self.owners.clone(),
                fallback_icon: self.cfg.fallback_icon.clone(),
            });
        }
        let Some(level) = self.level() else {
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
        // The chart is drawn only for a panel that is up. Off screen it is a
        // file nothing reads, redrawn every time the ring gains a point.
        if self.menu_open && !self.history.is_empty() {
            let key = graph::key(&self.history, self.cfg.span, self.cfg.dark);
            let drawing = self.graph_drawing.map(|(_, key)| key);
            if self.graph_drawn != Some(key)
                && drawing != Some(key)
                && self.graph_failed != Some(key)
            {
                let id = self.jobs.take();
                actions.push(ActionBattery::RenderGraph {
                    id,
                    points: self.history.clone(),
                    span: self.cfg.span,
                    dark: self.cfg.dark,
                });
                self.graph_drawing = Some((id, key));
            }
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let level = self.level().unwrap_or_default();
        out.set("batt_icon", self.icon());
        out.set("batt_pct", level.percent);
        out.set("batt_charging", level.charging);
        out.set("batt", self.detail());
        out.set("batt_graph", self.graph());
        out.set(
            "batt_range",
            format!("Last {} Hours", self.cfg.span / 3_600),
        );
        let rows: Vec<WireJson> = self
            .procs
            .iter()
            .map(|p| {
                WireJson::Obj(vec![
                    ("name", WireJson::s(&p.name)),
                    ("icon", WireJson::s(&p.icon)),
                    ("share", WireJson::s(format!("{}%", p.share))),
                    ("uptime", WireJson::s(wire_fmt::hm(p.uptime))),
                ])
            })
            .collect();
        out.set("batt_procs", WireJson::Arr(rows));
    }

    fn poll_interval(&self) -> Option<u32> {
        // An open panel is a live ranking and a moving estimate; a closed one is
        // a pill that changes a percent every few minutes.
        Some(if self.menu_open {
            self.cfg.menu_poll
        } else {
            self.cfg.poll
        })
    }
}

#[cfg(test)]
mod tests {
    use api::module::drive;

    use super::*;

    fn reading(percent: u8) -> Reading {
        Reading {
            battery: Battery {
                percent,
                charging: false,
            },
            ..Reading::default()
        }
    }

    fn observed(percent: u8) -> EventBattery {
        EventBattery::Observed {
            reading: reading(percent),
            history: Vec::new(),
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
        b.handle_event(&observed(50));
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
        let (actions, _) = drive(&mut b, observed(50));
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, ActionBattery::Render { dark: true, .. })),
            "expected a redraw for the new appearance, got {actions:?}"
        );
    }

    /// The expensive half of this module only runs for a panel that is up.
    #[test]
    fn the_list_is_only_sampled_while_the_panel_is_open() {
        let mut b = StateBattery::new(ConfigBattery::default());
        let (actions, _) = drive(&mut b, EventBattery::Due);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, ActionBattery::SampleProcs { .. })),
            "a closed panel must not time every process on the machine"
        );

        let (actions, _) = drive(&mut b, EventBattery::MenuOpened);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, ActionBattery::SampleProcs { .. })),
            "opening the panel asks for the list at once, not at the next tick"
        );
    }

    /// Same rule for the chart: it is a file nothing reads while the panel is
    /// closed, and the ring gains a point every minute.
    #[test]
    fn the_chart_is_only_drawn_while_the_panel_is_open() {
        let mut b = StateBattery::new(ConfigBattery::default());
        let history = vec![
            HistPoint {
                at: 1,
                percent: 80,
                charging: false,
            },
            HistPoint {
                at: 2,
                percent: 79,
                charging: false,
            },
        ];
        let (actions, _) = drive(
            &mut b,
            EventBattery::Observed {
                reading: reading(79),
                history: history.clone(),
            },
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, ActionBattery::RenderGraph { .. })),
            "a closed panel must not redraw the chart every sample"
        );

        let (actions, _) = drive(&mut b, EventBattery::MenuOpened);
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, ActionBattery::RenderGraph { .. })),
            "expected the chart to be drawn on open, got {actions:?}"
        );
        let _ = history;
    }

    /// An estimate the kernel cannot make must not be published as a duration.
    /// Zero seconds reads as "any moment now", which is the opposite of "no
    /// current is flowing, so there is nothing to divide".
    #[test]
    fn no_current_means_no_estimate() {
        let mut b = StateBattery::new(ConfigBattery::default());
        b.handle_event(&EventBattery::Observed {
            reading: Reading {
                battery: Battery {
                    percent: 100,
                    charging: true,
                },
                seconds_left: None,
                ..Reading::default()
            },
            history: Vec::new(),
        });
        let mut vars = WireVars::default();
        b.vars(&mut vars);
        let batt = vars.get("batt").unwrap_or_default();
        assert!(
            batt.contains(r#""time":"—""#),
            "expected a dash for an estimate that cannot be made, got {batt}"
        );
    }
}
