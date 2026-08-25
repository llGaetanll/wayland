//! The dock: a list of apps, reorderable by dragging one onto another.
//!
//! The list *is* the order — there is no separate registry to fall out of sync
//! with it — and the file on disk is written from this state, never read back
//! into it except at startup.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;
use crate::json::Json;
use crate::model::DockApp;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Dock {
    apps: Vec<DockApp>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Observed(Vec<DockApp>),
    Launch {
        id: String,
    },
    /// The user dropped app `a` onto app `b`; they trade places.
    Swap {
        a: String,
        b: String,
    },
}

impl Dock {
    pub fn apps(&self) -> &[DockApp] {
        &self.apps
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.apps.iter().position(|a| a.id == id)
    }
}

impl Widget for Dock {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::Observed(apps) => self.apps = apps,
            Ev::Launch { id } => {
                if let Some(app) = self.apps.iter().find(|a| a.id == id) {
                    cx.dispatch(ActionKind::Launch {
                        cmd: app.cmd.clone(),
                    });
                }
            }
            Ev::Swap { a, b } => {
                if a == b {
                    return;
                }
                let (Some(ia), Some(ib)) = (self.index_of(&a), self.index_of(&b)) else {
                    return;
                };
                self.apps.swap(ia, ib);
                cx.dispatch(ActionKind::PersistDock(self.apps.clone()));
            }
        }
    }

    fn vars(&self, _cfg: &Config, out: &mut Vars) {
        let apps: Vec<Json> = self
            .apps
            .iter()
            .map(|a| {
                Json::Obj(vec![
                    ("id", Json::s(&a.id)),
                    ("cmd", Json::s(&a.cmd)),
                    ("icon", Json::s(&a.icon)),
                ])
            })
            .collect();
        out.set("dock_apps", Json::Arr(apps));
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(Probe::Dock, cfg.poll.dock);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{kinds, run};

    fn apps(ids: &[&str]) -> Vec<DockApp> {
        ids.iter()
            .map(|id| DockApp {
                id: id.to_string(),
                cmd: id.to_string(),
                icon: format!("/i/{id}.svg"),
            })
            .collect()
    }

    fn ids(d: &Dock) -> Vec<&str> {
        d.apps.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn swap_reorders_and_persists() {
        let mut d = Dock::default();
        run(&mut d, Ev::Observed(apps(&["a", "b", "c"])));
        let acts = run(
            &mut d,
            Ev::Swap {
                a: "a".into(),
                b: "c".into(),
            },
        );
        assert_eq!(ids(&d), ["c", "b", "a"]);
        assert!(matches!(
            kinds(acts).first(),
            Some(ActionKind::PersistDock(_))
        ));
    }

    #[test]
    fn a_drop_on_itself_or_a_stranger_does_nothing() {
        let mut d = Dock::default();
        run(&mut d, Ev::Observed(apps(&["a", "b"])));
        assert!(kinds(run(
            &mut d,
            Ev::Swap {
                a: "a".into(),
                b: "a".into()
            }
        ))
        .is_empty());
        assert!(kinds(run(
            &mut d,
            Ev::Swap {
                a: "a".into(),
                b: "zz".into()
            }
        ))
        .is_empty());
        assert_eq!(ids(&d), ["a", "b"]);
    }
}
