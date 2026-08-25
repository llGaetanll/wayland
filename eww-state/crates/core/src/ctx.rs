//! The seam between a sub-machine and everything outside it.
//!
//! A sub-machine never reaches out: it can dispatch an action (which the
//! runtime will carry out) and it can raise a request (which its parent will
//! decide about). Both go through `Ctx`, so handling an event stays a pure
//! function of that machine's own state.

use std::collections::BTreeMap;
use std::time::Instant;

use crate::action::Action;
use crate::action_file::ActionFile;
use crate::action_probe::ActionProbe;
use crate::action_shell::ActionShell;
use crate::probe::JobId;
use crate::probe::Probe;

/// Which sub-machine a dispatched job belongs to, so its completion can be
/// routed back to the same place without the machines knowing about each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateId {
    Audio,
    Backlight,
    Wifi,
    Bluetooth,
    Battery,
    Date,
    Theme,
    Dock,
    Shot,
    Bar,
}

/// Something only the parent can grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Req {
    /// Dismiss whatever bar dropdown is open.
    CloseMenu,
}

pub struct Ctx<'a> {
    owner: StateId,
    now: Instant,
    next_job: &'a mut u64,
    actions: &'a mut Vec<Action>,
    owners: &'a mut BTreeMap<JobId, StateId>,
    reqs: &'a mut Vec<Req>,
}

impl<'a> Ctx<'a> {
    pub(crate) fn new(
        owner: StateId,
        now: Instant,
        next_job: &'a mut u64,
        actions: &'a mut Vec<Action>,
        owners: &'a mut BTreeMap<JobId, StateId>,
        reqs: &'a mut Vec<Req>,
    ) -> Self {
        Ctx {
            owner,
            now,
            next_job,
            actions,
            owners,
            reqs,
        }
    }

    /// The clock, as the last tick reported it. Reading this is not reading a
    /// clock: it is the value the runtime handed in, and a replay hands in the
    /// same one.
    pub fn now(&self) -> Instant {
        self.now
    }

    /// Ask for a sample now.
    ///
    /// Unlike the others this is dispatched unattributed: a probe's result
    /// arrives as its own event and nobody waits on the job. Routing its
    /// completion back would be a loop — a probe whose completion asks for
    /// another probe.
    pub fn probe(&mut self, what: Probe) -> JobId {
        let id = self.next_id();
        self.actions.push(Action::ToProbe {
            id,
            action: ActionProbe::Sample { what },
        });
        id
    }

    /// Queue a program to run. The returned id comes back on
    /// `EventJob::Finished`, so a machine can tell its own requests apart.
    pub fn shell(&mut self, action: ActionShell) -> JobId {
        let id = self.own_next_id();
        self.actions.push(Action::ToShell { id, action });
        id
    }

    pub fn file(&mut self, action: ActionFile) -> JobId {
        let id = self.own_next_id();
        self.actions.push(Action::ToFile { id, action });
        id
    }

    pub fn notify(&mut self, title: &str, body: impl Into<String>) {
        self.shell(ActionShell::Notify {
            title: title.to_string(),
            body: body.into(),
        });
    }

    pub fn request(&mut self, req: Req) {
        self.reqs.push(req);
    }

    fn next_id(&mut self) -> JobId {
        *self.next_job += 1;
        JobId(*self.next_job)
    }

    fn own_next_id(&mut self) -> JobId {
        let id = self.next_id();
        self.owners.insert(id, self.owner);
        id
    }
}
