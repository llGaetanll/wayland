//! The dock state machine. No IO.

use api::module::Module;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionDock;
use crate::config::ConfigDock;
use crate::event::EventDock;
use crate::model::DockApp;

/// The id the Trash tile clicks with. It is deliberately not a value anything
/// can put in `dock.conf`: a second entry by that name would be launchable but
/// would still not be the fixture the widget draws.
pub const TRASH: &str = "trash";

#[derive(Clone, Debug, Default)]
pub struct StateDock {
    cfg: ConfigDock,
    apps: Vec<DockApp>,
    /// Apps clicked this batch, in click order and deduplicated: a double click
    /// is one launch, two different icons are two.
    launching: Vec<String>,
    /// The order changed and the file has not caught up. A flag rather than a
    /// copy of the list, so two drags in one batch write once, with the order
    /// they ended on.
    unsaved: bool,
    want_sample: bool,
    want_running: bool,
    /// Ids the compositor has a window for, so the tile can show macOS's dot.
    running: Vec<String>,
    /// Ticks since the last disk resync. The module ticks at the running rate
    /// and counts up to the disk rate, rather than owning a second timer that
    /// could drift out of step with the first.
    ticks: u32,
    /// Whether the file has ever been read. Without this the resync counter
    /// starts at zero and the *first* read is an hour away, so the dock would
    /// come up empty and stay that way for the whole session.
    loaded: bool,
}

impl StateDock {
    pub fn new(cfg: ConfigDock) -> StateDock {
        StateDock {
            cfg,
            ..StateDock::default()
        }
    }

    pub fn apps(&self) -> &[DockApp] {
        &self.apps
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.apps.iter().position(|a| a.id == id)
    }
}

impl Module for StateDock {
    type Event = EventDock;
    type Action = ActionDock;
    type Req = ();

    fn handle_event(&mut self, event: &EventDock) {
        match event {
            // Every tick asks who is running, because that is what changes.
            // The order on disk changes only when we change it, so it is
            // re-read on a count of ticks rather than on its own timer.
            EventDock::Due => {
                self.want_running = true;
                self.ticks = self.ticks.saturating_add(1);
                let every = (self.cfg.poll / self.cfg.running_poll.max(1)).max(1);
                if !self.loaded || self.ticks >= every {
                    self.ticks = 0;
                    self.want_sample = true;
                }
            }
            EventDock::Running { ids } => self.running = ids.clone(),
            // A resync cannot overwrite an order we have not written back yet;
            // the file is behind us, not ahead.
            EventDock::Observed { apps } => {
                self.loaded = true;
                if !self.unsaved {
                    self.apps = apps.clone();
                }
            }
            // The Trash answers to a click like an app does, but it is not in
            // the list, so `known` has to admit it by name or the click would
            // be dropped as coming from a list that had moved.
            EventDock::Launched { id } => {
                let known = id == TRASH || self.apps.iter().any(|a| &a.id == id);
                if known && !self.launching.contains(id) {
                    self.launching.push(id.clone());
                }
            }
            EventDock::Dropped { moved, onto } => {
                if moved == onto {
                    return;
                }
                let (Some(a), Some(b)) = (self.index_of(moved), self.index_of(onto)) else {
                    return;
                };
                self.apps.swap(a, b);
                self.unsaved = true;
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionDock>) -> Option<()> {
        for id in std::mem::take(&mut self.launching) {
            let cmd = if id == TRASH {
                self.cfg.trash_cmd.clone()
            } else {
                match self.apps.iter().find(|a| a.id == id) {
                    Some(app) => app.cmd.clone(),
                    None => continue, // the list moved under the click
                }
            };
            actions.push(ActionDock::Launch { cmd });
        }
        if std::mem::take(&mut self.unsaved) {
            actions.push(ActionDock::Write {
                apps: self.apps.clone(),
            });
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionDock::Sample);
        }
        if std::mem::take(&mut self.want_running) {
            actions.push(ActionDock::SampleRunning {
                ids: self.apps.iter().map(|a| a.id.clone()).collect(),
            });
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let apps: Vec<WireJson> = self
            .apps
            .iter()
            .map(|a| {
                WireJson::Obj(vec![
                    ("id", WireJson::s(&a.id)),
                    ("cmd", WireJson::s(&a.cmd)),
                    ("icon", WireJson::s(&a.icon)),
                    ("running", WireJson::Bool(self.running.contains(&a.id))),
                ])
            })
            .collect();
        out.set("dock_apps", WireJson::Arr(apps));
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.running_poll)
    }
}

#[cfg(test)]
mod tests {
    use api::module::drive;
    use api::module::Module;

    use super::*;

    fn app(id: &str) -> DockApp {
        DockApp {
            id: id.to_string(),
            cmd: id.to_string(),
            icon: String::new(),
        }
    }

    fn state() -> StateDock {
        StateDock::new(ConfigDock::default())
    }

    /// The disk resync rides on a count of ticks, so the very first tick has to
    /// be the one that reads the file — otherwise the dock is empty until the
    /// hour is up.
    #[test]
    fn the_first_tick_reads_the_file() {
        let (actions, _) = drive(&mut state(), EventDock::Due);
        assert!(
            actions.iter().any(|a| matches!(a, ActionDock::Sample)),
            "expected a disk read on the first tick, got {actions:?}"
        );
    }

    /// ...and the second does not, or the lazy resync would not be lazy.
    #[test]
    fn a_later_tick_only_asks_who_is_running() {
        let mut dock = state();
        drive(&mut dock, EventDock::Due);
        drive(
            &mut dock,
            EventDock::Observed {
                apps: vec![app("firefox")],
            },
        );
        let (actions, _) = drive(&mut dock, EventDock::Due);
        assert!(!actions.iter().any(|a| matches!(a, ActionDock::Sample)));
        assert!(actions
            .iter()
            .any(|a| matches!(a, ActionDock::SampleRunning { .. })));
    }

    #[test]
    fn running_ids_reach_the_widget() {
        let mut dock = state();
        dock.handle_event(&EventDock::Observed {
            apps: vec![app("firefox"), app("signal")],
        });
        dock.handle_event(&EventDock::Running {
            ids: vec!["firefox".to_string()],
        });
        let mut vars = api::wire_vars::WireVars::default();
        dock.vars(&mut vars);
        let json = format!("{vars:?}");
        assert!(json.contains("firefox"), "{json}");
    }

    /// The Trash is not in the list, so a click on it has to be admitted by
    /// name or it would be dropped as coming from a stale list.
    #[test]
    fn the_trash_launches_even_though_it_is_not_an_app() {
        let mut dock = state();
        dock.handle_event(&EventDock::Observed { apps: vec![] });
        let (actions, _) = drive(
            &mut dock,
            EventDock::Launched {
                id: TRASH.to_string(),
            },
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, ActionDock::Launch { cmd } if cmd.contains("trash"))),
            "got {actions:?}"
        );
    }
}
