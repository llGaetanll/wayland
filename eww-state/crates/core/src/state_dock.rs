//! The dock: a list of apps, reorderable by dragging one onto another.
//!
//! The list *is* the order — there is no separate registry to fall out of sync
//! with it — and the file on disk is written from this state, never read back
//! into it except at startup.

use crate::action::Action;
use crate::action_file::ActionFile;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::model::DockApp;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_json::WireJson;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateDock {
    apps: Vec<DockApp>,
    /// Apps clicked this batch, in click order and deduplicated: a double click
    /// is one launch, two different icons are two.
    launching: Vec<String>,
    /// The order changed and the file on disk has not caught up. A flag rather
    /// than a copy of the list, so two drags in one batch write once, with the
    /// order they ended on.
    unsaved: bool,
}

#[derive(Debug)]
pub enum EventDock {
    Observed {
        apps: Vec<DockApp>,
    },
    Launch {
        id: String,
    },
    /// The user dropped app `moved` onto app `onto`; they trade places.
    Swap {
        moved: String,
        onto: String,
    },
}

impl StateDock {
    pub fn apps(&self) -> &[DockApp] {
        &self.apps
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.apps.iter().position(|a| a.id == id)
    }
}

impl State for StateDock {
    type Event = EventDock;

    fn handle_event(&mut self, _cfg: &Config, event: EventDock) {
        match event {
            // A resync from disk cannot overwrite an order we have not written
            // back yet; the file is behind us, not ahead.
            EventDock::Observed { apps } => {
                if !self.unsaved {
                    self.apps = apps;
                }
            }
            EventDock::Launch { id } => {
                let known = self.apps.iter().any(|a| a.id == id);
                if known && !self.launching.contains(&id) {
                    self.launching.push(id);
                }
            }
            EventDock::Swap { moved, onto } => {
                if moved == onto {
                    return;
                }
                let (Some(a), Some(b)) = (self.index_of(&moved), self.index_of(&onto)) else {
                    return;
                };
                self.apps.swap(a, b);
                self.unsaved = true;
            }
        }
    }

    fn emit_actions(
        &mut self,
        _cfg: &Config,
        jobs: &mut Jobs,
        actions: &mut Vec<Action>,
    ) -> Option<Req> {
        for id in std::mem::take(&mut self.launching) {
            let Some(app) = self.apps.iter().find(|a| a.id == id) else {
                continue; // the list moved under the click
            };
            actions.push(Action::shell(
                jobs.take(),
                ActionShell::Launch {
                    cmd: app.cmd.clone(),
                },
            ));
        }
        if self.unsaved {
            self.unsaved = false;
            actions.push(Action::file(
                jobs.take(),
                ActionFile::WriteDock {
                    apps: self.apps.clone(),
                },
            ));
        }
        None
    }

    fn vars(&self, _cfg: &Config, out: &mut WireVars) {
        let apps: Vec<WireJson> = self
            .apps
            .iter()
            .map(|a| {
                WireJson::Obj(vec![
                    ("id", WireJson::s(&a.id)),
                    ("cmd", WireJson::s(&a.cmd)),
                    ("icon", WireJson::s(&a.icon)),
                ])
            })
            .collect();
        out.set("dock_apps", WireJson::Arr(apps));
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Dock, cfg.poll.dock);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::test_util::run;

    fn apps(ids: &[&str]) -> Vec<DockApp> {
        ids.iter()
            .map(|id| DockApp {
                id: id.to_string(),
                cmd: id.to_string(),
                icon: format!("/i/{id}.svg"),
            })
            .collect()
    }

    fn ids(d: &StateDock) -> Vec<&str> {
        d.apps.iter().map(|a| a.id.as_str()).collect()
    }

    fn swap(moved: &str, onto: &str) -> EventDock {
        EventDock::Swap {
            moved: moved.into(),
            onto: onto.into(),
        }
    }

    #[test]
    fn swap_reorders_and_persists() {
        let mut d = StateDock::default();
        run(
            &mut d,
            EventDock::Observed {
                apps: apps(&["a", "b", "c"]),
            },
        );
        let actions = run(&mut d, swap("a", "c"));
        assert_eq!(ids(&d), ["c", "b", "a"]);
        assert!(matches!(
            actions.first(),
            Some(Action::ToFile {
                action: ActionFile::WriteDock { .. },
                ..
            })
        ));
    }

    #[test]
    fn a_drop_on_itself_or_a_stranger_does_nothing() {
        let mut d = StateDock::default();
        run(
            &mut d,
            EventDock::Observed {
                apps: apps(&["a", "b"]),
            },
        );
        assert!(run(&mut d, swap("a", "a")).is_empty());
        assert!(run(&mut d, swap("a", "zz")).is_empty());
        assert_eq!(ids(&d), ["a", "b"]);
    }
}
