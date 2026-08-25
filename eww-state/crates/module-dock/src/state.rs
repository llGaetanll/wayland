//! The dock state machine. No IO.

use api::module::Module;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionDock;
use crate::config::ConfigDock;
use crate::event::EventDock;
use crate::model::DockApp;

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
            EventDock::Due => self.want_sample = true,
            // A resync cannot overwrite an order we have not written back yet;
            // the file is behind us, not ahead.
            EventDock::Observed { apps } => {
                if !self.unsaved {
                    self.apps = apps.clone();
                }
            }
            EventDock::Launched { id } => {
                let known = self.apps.iter().any(|a| &a.id == id);
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
            let Some(app) = self.apps.iter().find(|a| a.id == id) else {
                continue; // the list moved under the click
            };
            actions.push(ActionDock::Launch {
                cmd: app.cmd.clone(),
            });
        }
        if std::mem::take(&mut self.unsaved) {
            actions.push(ActionDock::Write {
                apps: self.apps.clone(),
            });
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionDock::Sample);
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
                ])
            })
            .collect();
        out.set("dock_apps", WireJson::Arr(apps));
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(self.cfg.poll)
    }
}
