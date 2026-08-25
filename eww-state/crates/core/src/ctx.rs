//! The seam between a widget machine and everything outside it.
//!
//! A widget never reaches out: it can dispatch an action (which the shell will
//! run) and it can raise a request (which its parent will decide about). Both go
//! through `Ctx`, so a widget's `update` stays a pure function of its own state.

use std::collections::BTreeMap;
use std::fmt::Display;

use crate::action::{Action, ActionKind};
use crate::event::JobId;

/// Which widget machine a dispatched job belongs to, so its completion can be
/// routed back to the same place without the widgets knowing about each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WidgetId {
    Audio,
    Backlight,
    Wifi,
    Bluetooth,
    Battery,
    Clock,
    Theme,
    Dock,
    Shot,
    Bar,
}

/// Something only the parent can grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Req {
    /// Dismiss whatever bar dropdown is open.
    CloseMenu,
}

pub struct Ctx<'a> {
    owner: WidgetId,
    next_job: &'a mut u64,
    actions: &'a mut Vec<Action>,
    owners: &'a mut BTreeMap<JobId, WidgetId>,
    reqs: &'a mut Vec<Req>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        owner: WidgetId,
        next_job: &'a mut u64,
        actions: &'a mut Vec<Action>,
        owners: &'a mut BTreeMap<JobId, WidgetId>,
        reqs: &'a mut Vec<Req>,
    ) -> Self {
        Ctx {
            owner,
            next_job,
            actions,
            owners,
            reqs,
        }
    }

    /// Queue a side effect. The returned id comes back on `Event::JobDone`, so a
    /// widget can tell its own outstanding requests apart.
    ///
    /// Probes are the exception: their result arrives as an observation, and
    /// nobody waits on the job itself. Routing their completion back to the
    /// widget that asked would be a loop — a probe whose completion asks for
    /// another probe — so they are dispatched unattributed.
    pub fn dispatch(&mut self, kind: ActionKind) -> JobId {
        *self.next_job += 1;
        let id = JobId(*self.next_job);
        if !matches!(kind, ActionKind::Probe(_)) {
            self.owners.insert(id, self.owner);
        }
        self.actions.push(Action { id, kind });
        id
    }

    pub fn notify(&mut self, title: &str, body: impl Into<String>) {
        self.dispatch(ActionKind::Notify {
            title: title.to_string(),
            body: body.into(),
        });
    }

    pub fn request(&mut self, req: Req) {
        self.reqs.push(req);
    }
}

/// The eww variables a widget publishes. The shell diffs this against what it
/// last sent, so a widget can republish freely and only real changes hit eww.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vars(BTreeMap<&'static str, String>);

impl Vars {
    pub fn set(&mut self, name: &'static str, value: impl Display) {
        self.0.insert(name, value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> + '_ {
        self.0.iter().map(|(k, v)| (*k, v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
