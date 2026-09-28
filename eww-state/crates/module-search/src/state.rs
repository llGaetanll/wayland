//! The launcher's rules.
//!
//! The text lives in eww's own entry rather than here, and arrives as an event
//! on every change. That is not a shortcut: the round trip from a keystroke to
//! a published variable and back is about 30ms, so a machine that owned the
//! buffer and echoed it back would draw the letters a frame or two behind the
//! typing, and a slow echo landing after the next keystroke would delete it.
//! What this owns is everything the entry cannot: the index, the ranking, the
//! highlight, and whether the panel is up at all.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::module::Module;
use api::util::b64;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionSearch;
use crate::config::ConfigSearch;
use crate::event::EventSearch;
use crate::matcher::rank;
use crate::model::App;

#[derive(Clone, Debug, Default)]
pub struct StateSearch {
    cfg: ConfigSearch,
    jobs: Jobs,
    open: bool,
    apps: Vec<App>,
    query: String,
    /// Which result is highlighted, as a position in the current ranking.
    /// Nothing derived is stored, so this is an index rather than a copy of the
    /// app it points at — a stale copy could name something no longer listed.
    selected: usize,
    indexing: Option<JobId>,
    /// The index wants rebuilding. Set every time the panel opens, because apps
    /// are installed and removed while a session is up.
    stale: bool,
    /// Return was pressed. Which app that launches is decided once the batch is
    /// over, so the change event for the last letter typed is already in.
    accepting: bool,
    /// What to launch once the batch is over.
    launch: Option<String>,
    /// Whether the compositor has already been told to send us the keys the
    /// text field does not want.
    grabbed: bool,
}

impl StateSearch {
    pub fn new(cfg: ConfigSearch) -> StateSearch {
        StateSearch {
            cfg,
            ..StateSearch::default()
        }
    }

    /// The panel is on screen.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The current ranking. Derived on every call rather than stored: a kept
    /// copy is a copy that can disagree with the query it came from.
    fn hits(&self) -> Vec<&App> {
        rank(&self.apps, &self.query, self.cfg.max_results)
    }

    fn set_query(&mut self, text: &str) {
        if self.query == text {
            return;
        }
        self.query = text.to_string();
        // A new ranking makes the old position meaningless.
        self.selected = 0;
    }

    fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
    }
}

impl Module for StateSearch {
    type Event = EventSearch;
    type Action = ActionSearch;
    type Req = ();

    fn handle_event(&mut self, event: &EventSearch) {
        match event {
            EventSearch::Toggled => {
                if self.open {
                    self.close();
                } else {
                    self.open = true;
                    self.query.clear();
                    self.selected = 0;
                    self.stale = true;
                }
            }
            EventSearch::Dismissed => self.close(),
            // A change from a panel that is already down is the entry being
            // torn apart as its window closes, not something the user typed.
            EventSearch::Queried { text } => {
                if self.open {
                    self.set_query(text);
                }
            }
            EventSearch::Moved { delta } => {
                let n = self.hits().len();
                if n == 0 {
                    return;
                }
                let last = n.saturating_sub(1) as i64;
                let to = (self.selected as i64).saturating_add(*delta as i64);
                self.selected = to.clamp(0, last) as usize;
            }
            EventSearch::Accepted { text } => {
                if self.open {
                    self.set_query(text);
                    self.accepting = true;
                }
            }
            EventSearch::Picked { exec } => {
                self.launch = Some(exec.clone());
                self.close();
            }
            EventSearch::Indexed { apps } => {
                self.apps = apps.clone();
                let last = self.hits().len().saturating_sub(1);
                self.selected = self.selected.min(last);
            }
            EventSearch::Finished { id } => {
                if self.indexing == Some(*id) {
                    self.indexing = None;
                }
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionSearch>) -> Option<()> {
        if self.open && self.stale && self.indexing.is_none() {
            let id = self.jobs.take();
            self.stale = false;
            self.indexing = Some(id);
            actions.push(ActionSearch::Index { id });
        }
        if std::mem::take(&mut self.accepting) {
            let picked = self.hits().get(self.selected).map(|a| a.exec.clone());
            // Return on an empty field, or on one matching nothing, puts the
            // panel away rather than doing nothing: there is no result to pick
            // and leaving it up would look like the key was swallowed.
            self.launch = picked;
            self.close();
        }
        if let Some(exec) = self.launch.take() {
            actions.push(ActionSearch::Launch { exec });
        }
        if self.grabbed != self.open {
            self.grabbed = self.open;
            actions.push(ActionSearch::Keys {
                grabbed: self.grabbed,
            });
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let hits = self.hits();
        let rows: Vec<WireJson> = hits
            .iter()
            .enumerate()
            .map(|(i, a)| {
                WireJson::Obj(vec![
                    ("name", WireJson::s(&a.name)),
                    ("icon", WireJson::s(&a.icon)),
                    // The command reaches a widget handler as part of a shell
                    // command line, so it travels encoded, like an SSID.
                    ("exec_b64", WireJson::s(b64(&a.exec))),
                    ("sel", WireJson::Bool(i == self.selected)),
                ])
            })
            .collect();
        out.set("search_hits", WireJson::Arr(rows));
        out.set("search_h", self.cfg.list_height(hits.len()));
        // Something was typed and nothing matched it, which is the one case
        // that wants a row saying so rather than a panel that just stops.
        out.set(
            "search_none",
            !self.query.trim().is_empty() && hits.is_empty(),
        );
    }
}
