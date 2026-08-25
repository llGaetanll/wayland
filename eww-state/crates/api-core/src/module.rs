//! What every module is, from the outside.
//!
//! A module owns one widget: its state, its event alphabet, the actions it can
//! ask for, and the IO that carries those out. It names no other module and no
//! parent. Everything it cannot do for itself — dismiss a dropdown, be told the
//! time — either arrives as an event or leaves as a request.
//!
//! The two calls are the whole contract, and the order matters: every event in
//! a batch is applied before anything is asked of the module, because what a
//! batch adds up to is not knowable until all of it has been seen. See
//! `docs/STATE_MACHINES.md`.

use crate::wire_vars::WireVars;

pub trait Module {
    /// This module's private event alphabet.
    type Event;
    /// What it can ask the outside world to do.
    type Action;
    /// What it can ask its parent for. `()` when it never asks for anything.
    type Req;

    /// Apply every event in the batch. Moves state and nothing else.
    ///
    /// Provided, because it is always the same loop — the parent hands down a
    /// slice it owns, so a replay can feed the same buffer back in.
    fn handle_events(&mut self, events: &[Self::Event]) {
        for event in events {
            self.handle_event(event);
        }
    }

    /// Apply one event. Moves state and nothing else.
    fn handle_event(&mut self, event: &Self::Event);

    /// Say what has to happen now, given everything the batch left behind.
    ///
    /// `&mut self` because actions are not idempotent: what stops the same
    /// request going out twice is this module recording that it already asked.
    fn emit_actions(&mut self, actions: &mut Vec<Self::Action>) -> Option<Self::Req>;

    /// Publish the eww vars this module owns. A pure function of state: the
    /// runtime may call it at any time and diffs the result.
    fn vars(&self, out: &mut WireVars);

    /// How often this module wants sampling right now, in milliseconds — an
    /// open menu wants fresher data than a bar icon does. `None` means never on
    /// a timer.
    fn poll_interval(&self) -> Option<u32> {
        None
    }
}

/// Drive a module the way its parent does: apply the event, then ask what it
/// wants. Two calls, in that order, which is the whole contract — so a test
/// that uses this is exercising the same sequence the program does.
pub fn drive<M: Module>(module: &mut M, event: M::Event) -> (Vec<M::Action>, Option<M::Req>) {
    let mut actions = Vec::new();
    module.handle_event(&event);
    let req = module.emit_actions(&mut actions);
    (actions, req)
}
