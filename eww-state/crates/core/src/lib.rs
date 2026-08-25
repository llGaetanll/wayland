//! The eww widget state machine.
//!
//! This crate decides *what the desktop widgets should look like and what
//! should happen next*. It performs no IO: it does not run commands, open
//! sockets, read files or look at a clock. Facts about the world arrive as
//! [`Event`]s and requests to change the world leave as [`Action`]s, which the
//! daemon crate carries out.
//!
//! That split is the point. The old shell-script version had every widget
//! talking to eww directly, so overlapping handlers wrote conflicting state and
//! a killed script could leave a window open that nothing would close again.
//! Here, all state lives in one machine, events are applied one at a time in
//! order, and what should be on screen is *derived* from the state rather than
//! remembered alongside it.
//!
//! Layout: one machine per widget in [`widgets`], each owning its own state and
//! event alphabet, and [`Bar`] on top routing between them and owning the one
//! cross-widget fact — which dropdown is open.

pub mod action;
pub mod bar;
pub mod config;
pub mod ctx;
pub mod event;
pub mod json;
pub mod model;
pub mod util;
pub mod widgets;
pub mod window;

pub use action::{Action, ActionKind};
pub use bar::Bar;
pub use config::Config;
pub use ctx::{Req, Vars, WidgetId};
pub use event::{Event, Intent, JobId, Obs, Probe};
pub use model::*;
pub use widgets::Widget;
pub use window::Window;

#[cfg(test)]
pub(crate) mod test_util {
    use std::collections::BTreeMap;

    use crate::action::{Action, ActionKind};
    use crate::ctx::{Ctx, Req, WidgetId};
    use crate::widgets::Widget;

    /// Drive a widget in isolation and collect what it asked for.
    pub fn run<W: Widget>(w: &mut W, ev: W::Ev) -> Vec<Action> {
        run_full(w, ev).0
    }

    pub fn run_full<W: Widget>(w: &mut W, ev: W::Ev) -> (Vec<Action>, Vec<Req>) {
        let mut next = 0u64;
        let mut actions = Vec::new();
        let mut owners = BTreeMap::new();
        let mut reqs = Vec::new();
        {
            let mut cx = Ctx::new(
                WidgetId::Bar,
                &mut next,
                &mut actions,
                &mut owners,
                &mut reqs,
            );
            w.update(ev, &mut cx);
        }
        (actions, reqs)
    }

    pub fn kinds(actions: Vec<Action>) -> Vec<ActionKind> {
        actions.into_iter().map(|a| a.kind).collect()
    }
}
