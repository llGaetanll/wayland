//! The Spaces state machine. No IO.
//!
//! Every rule here is decided against a *settled* look at the compositor rather
//! than against the event that woke us, which is what removed the timers this
//! logic used to need.
//!
//! The trap it exists for: moving a fullscreen window makes the compositor
//! re-emit fullscreen — a transient `0` followed by a `1` — and that `0` reads
//! exactly like the user leaving fullscreen. Act on it and the window is moved
//! home, which emits the echo again, forever. The Lua version swallowed those
//! with a 140ms debounce and hoped.
//!
//! They need no debounce, because the echoes are ours: the module knows it has
//! a move in flight, and a look taken while one is cannot be believed. So a
//! move suspends every rule until it finishes, and the first look afterwards is
//! the first one trusted. Nothing here reads a clock.

use std::collections::BTreeMap;

use api::jobs::JobId;
use api::jobs::Jobs;
use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionSpaces;
use crate::event::EventSpaces;
use crate::model::MoveTarget;
use crate::model::Snapshot;
use crate::model::WindowAddr;
use crate::model::WorkspaceId;

#[derive(Debug, Default)]
pub struct StateSpaces {
    jobs: Jobs,
    /// Where each window in a Space came from. Absent means *not in a Space* —
    /// this map is the whole of what used to be a directory of files, and being
    /// a map it cannot go stale behind a window's back.
    origins: BTreeMap<WindowAddr, WorkspaceId>,
    /// The last look worth believing.
    layout: Snapshot,
    /// Moves we asked for and have not seen finish, by the job carrying each.
    moving: BTreeMap<JobId, WindowAddr>,
    /// Looks older than this were taken while a move of ours was in flight, so
    /// what they say about fullscreen is the compositor echoing us back.
    /// `None` until a move has ever happened, when every look is honest.
    trust_from: Option<JobId>,
    /// A move finished, so the next look is the one to start trusting.
    settling: bool,
    /// A look the batch asked for, collapsed: a burst of compositor events is
    /// one question.
    due: bool,
    /// A look already in flight, so a burst does not become a burst of them.
    looking: Option<JobId>,
    closing: Option<Closing>,
}

/// Closing takes two turns, because which window is focused is the one thing a
/// look taken before the keypress cannot be trusted about.
#[derive(Clone, Copy, Debug)]
enum Closing {
    /// The keypress landed.
    Wanted,
    /// A look was sent for it; this is the one to wait for.
    Awaiting(JobId),
    /// It arrived, so the focused window is known.
    Ready,
}

impl StateSpaces {
    /// The focused workspace holds a fullscreen window, so the bar and dock
    /// should get out of the way.
    ///
    /// Derived from the same look the Space rules are decided on, so what is on
    /// screen and where windows are cannot disagree about what the compositor
    /// said.
    pub fn fullscreen(&self) -> bool {
        self.layout.fullscreen()
    }

    /// This window is in a Space of its own.
    pub fn in_space(&self, addr: &WindowAddr) -> bool {
        self.origins.contains_key(addr)
    }

    /// Ask the compositor what it looks like now.
    fn look(&mut self, actions: &mut Vec<ActionSpaces>) -> JobId {
        let id = self.jobs.take();
        debug_assert!(
            self.trust_from.is_none_or(|t| id >= t),
            "job ids went backwards"
        );
        if std::mem::take(&mut self.settling) {
            self.trust_from = Some(id);
        }
        self.looking = Some(id);
        actions.push(ActionSpaces::Look { id });
        id
    }

    fn observed(&mut self, id: JobId, snapshot: &Snapshot) {
        // An answer is proof the look is over, believed or not — otherwise a
        // look dropped for arriving mid-move would block every later one.
        if self.looking == Some(id) {
            self.looking = None;
        }
        // Mid-cut, or taken while it was: what the compositor says about
        // fullscreen during a move of ours is the move, not the user.
        if !self.moving.is_empty() || self.trust_from.is_some_and(|t| id < t) {
            return;
        }
        if let Some(Closing::Awaiting(waited)) = self.closing {
            if id >= waited {
                self.closing = Some(Closing::Ready);
            }
        }
        self.layout = snapshot.clone();
        // A window that has gone takes its origin with it. This is the pruning
        // sweep the shell version ran over a directory, except that there is
        // nothing left behind to sweep.
        self.origins
            .retain(|addr, _| snapshot.client(addr).is_some());
    }

    fn finished(&mut self, id: JobId) {
        if self.looking == Some(id) {
            self.looking = None;
            // A look that produced nothing cannot be waited on forever.
            if matches!(self.closing, Some(Closing::Awaiting(w)) if w == id) {
                self.closing = None;
            }
        }
        if self.moving.remove(&id).is_some() && self.moving.is_empty() {
            self.settling = true;
            self.due = true;
        }
    }

    /// Put every window where the rule says it belongs.
    ///
    /// Skipped entirely while a move of ours is in flight: the compositor is
    /// mid-cut, and a `Sampled` from then is dropped rather than acted on.
    fn settle(&mut self, actions: &mut Vec<ActionSpaces>) {
        if !self.moving.is_empty() {
            return;
        }
        let mut entering = Vec::new();
        let mut leaving = Vec::new();
        for client in &self.layout.clients {
            match (client.fullscreen, self.origins.contains_key(&client.addr)) {
                (true, false) => entering.push((client.addr.clone(), client.workspace)),
                (false, true) => leaving.push(client.addr.clone()),
                // Already in a Space, or an ordinary window that was never in
                // one. Both are the common case and both are nothing to do.
                (true, true) | (false, false) => {}
            }
        }
        for (addr, from) in entering {
            self.origins.insert(addr.clone(), from);
            self.dispatch_move(addr, MoveTarget::Empty, actions);
        }
        for addr in leaving {
            if let Some(workspace) = self.origins.remove(&addr) {
                self.dispatch_move(addr, MoveTarget::Workspace(workspace), actions);
            }
        }
    }

    fn dispatch_move(&mut self, addr: WindowAddr, to: MoveTarget, actions: &mut Vec<ActionSpaces>) {
        let id = self.jobs.take();
        self.moving.insert(id, addr.clone());
        actions.push(ActionSpaces::Move { id, addr, to });
    }
}

impl Module for StateSpaces {
    type Event = EventSpaces;
    type Action = ActionSpaces;
    type Req = ();

    fn handle_event(&mut self, event: &EventSpaces) {
        match event {
            EventSpaces::Due => self.due = true,
            EventSpaces::CloseRequested => self.closing = Some(Closing::Wanted),
            EventSpaces::Sampled { id, snapshot } => self.observed(*id, snapshot),
            EventSpaces::Finished { id } => self.finished(*id),
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionSpaces>) -> Option<()> {
        if matches!(self.closing, Some(Closing::Wanted)) {
            // Deliberately not reusing a look already in flight: that one was
            // sent for before the keypress, and could name the window the user
            // was on a moment ago.
            let id = self.look(actions);
            self.closing = Some(Closing::Awaiting(id));
        }
        if matches!(self.closing, Some(Closing::Ready)) {
            self.closing = None;
            if let Some(addr) = self.layout.active.clone() {
                // The origin stays recorded until the window is actually gone,
                // and the next look prunes it. Dropping it here would leave a
                // window that is still fullscreen with no Space of its own,
                // and the rules below would put it in a new one on the way out.
                if let Some(workspace) = self.origins.get(&addr).copied() {
                    actions.push(ActionSpaces::Focus { workspace });
                }
                actions.push(ActionSpaces::Close { addr });
            }
        }
        if std::mem::take(&mut self.due) && self.looking.is_none() {
            self.look(actions);
        }
        self.settle(actions);
        None
    }

    /// Nothing. What this module owns is where windows are, and eww is told
    /// about that by the bar deciding it has to get out of the way.
    fn vars(&self, _out: &mut WireVars) {}
}
