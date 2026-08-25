//! The dock: a list of apps, reorderable by dragging one onto another.
//!
//! The list *is* the order — there is no separate registry to fall out of sync
//! with it — and the file on disk is written from this state, never read back
//! into it except at startup.

use crate::action_file::ActionFile;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::ctx::Ctx;
use crate::model::DockApp;
use crate::probe::Probe;
use crate::state::PollPlan;
use crate::state::State;
use crate::wire_json::WireJson;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateDock {
    apps: Vec<DockApp>,
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

    fn handle_event(&mut self, _cfg: &Config, event: EventDock, cx: &mut Ctx) {
        match event {
            EventDock::Observed { apps } => self.apps = apps,
            EventDock::Launch { id } => {
                if let Some(app) = self.apps.iter().find(|a| a.id == id) {
                    cx.shell(ActionShell::Launch {
                        cmd: app.cmd.clone(),
                    });
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
                cx.file(ActionFile::WriteDock {
                    apps: self.apps.clone(),
                });
            }
        }
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
