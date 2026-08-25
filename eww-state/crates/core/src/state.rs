//! What every sub-machine has in common.
//!
//! A sub-machine owns one widget's state and nothing else. It does not handle
//! events itself: [`StateBar`](crate::state_bar::StateBar) translates the shared
//! `Event` vocabulary into that machine's private alphabet and routes it here,
//! so there is still exactly one way anything changes.
//!
//! Those private alphabets live beside their machine rather than in their own
//! file. The file-per-group rule is about the vocabulary the whole program
//! shares; `EventWifi` is read by nothing but `StateWifi`.

use std::collections::BTreeMap;

use crate::action::Action;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::probe::Probe;
use crate::req::Req;
use crate::wire_vars::WireVars;

/// Interval, in milliseconds, at which a machine wants a probe sampled. Zero
/// means never on a timer — that probe runs only when something asks for it.
pub type PollPlan = BTreeMap<Probe, u32>;

pub trait State {
    /// This machine's private event alphabet.
    type Event;

    /// Apply one event. This moves state and nothing else: no action may be
    /// emitted here, because the rest of the batch has not been seen yet.
    fn handle_event(&mut self, cfg: &Config, event: Self::Event);

    /// Say what has to happen now, given everything the batch left behind.
    ///
    /// `&mut self` because actions are not idempotent: what stops the same
    /// request going out twice is this machine recording that it already asked.
    /// Anything only the parent can grant comes back as the return value.
    fn emit_actions(
        &mut self,
        _cfg: &Config,
        _jobs: &mut Jobs,
        _actions: &mut Vec<Action>,
    ) -> Option<Req> {
        None
    }

    /// Publish the eww vars this machine owns. Must be a pure function of
    /// state: the runtime may call it at any time and diffs the result.
    fn vars(&self, cfg: &Config, out: &mut WireVars);

    /// How often this machine wants its data sampled, given its current state —
    /// an open menu polls faster than a bar icon does.
    fn polls(&self, _cfg: &Config, _out: &mut PollPlan) {}
}
