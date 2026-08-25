//! One state machine per widget.
//!
//! Each module here owns a single widget's state, its own event type, and the
//! eww vars it publishes — and nothing else. Widgets never call each other; the
//! bar machine in [`crate::bar`] routes between them, and anything a widget
//! needs from the outside world it asks for through [`Ctx`].

use std::collections::BTreeMap;

use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::Probe;

pub mod audio;
pub mod backlight;
pub mod battery;
pub mod bluetooth;
pub mod clock;
pub mod dock;
pub mod shot;
pub mod theme;
pub mod wifi;

/// Interval, in milliseconds, at which a widget wants a probe sampled.
pub type PollPlan = BTreeMap<Probe, u32>;

pub trait Widget {
    /// This widget's private event alphabet.
    type Ev;

    fn update(&mut self, ev: Self::Ev, cx: &mut Ctx);

    /// Publish the eww vars this widget owns. Must be a pure function of state:
    /// the shell may call it at any time and diffs the result.
    fn vars(&self, cfg: &Config, out: &mut Vars);

    /// How often this widget wants its data sampled, given its current state —
    /// an open menu polls faster than a bar icon does.
    fn polls(&self, _cfg: &Config, _out: &mut PollPlan) {}
}
