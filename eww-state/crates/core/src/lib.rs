//! The eww widget state machine.
//!
//! This crate decides *what the desktop widgets should look like and what
//! should happen next*. It performs no IO: it does not run commands, open
//! sockets, read files or look at a clock. Facts about the world arrive as
//! [`Event`]s — including the time, which arrives as a tick — and requests to
//! change the world leave as [`Action`]s, which the daemon crate carries out.
//!
//! That split is the point. The old shell-script version had every widget
//! talking to eww directly, so overlapping handlers wrote conflicting state and
//! a killed script could leave a window open that nothing would close again.
//! Here, all state lives in one machine, events are applied in order, and what
//! should be on screen is *derived* from the state rather than remembered
//! alongside it.
//!
//! Layout: one machine per widget, each owning its own state and event
//! alphabet, and [`StateBar`] on top routing between them and owning the one
//! cross-widget fact — which dropdown is open.

/// Assert that a wire value serializes to exactly this text.
///
/// The expected string is the contract, so it is written out rather than
/// snapshotted. See the wire-type rules in docs/STYLE.md.
#[cfg(test)]
macro_rules! serializes {
    ($value:expr, $expected:expr) => {
        assert_eq!($value.to_string(), $expected);
    };
}
pub mod action;
pub mod action_file;
pub mod action_probe;
pub mod action_shell;
pub mod config;
pub mod ctx;
pub mod event;
pub mod event_clock;
pub mod event_eww;
pub mod event_job;
pub mod event_probe;
pub mod event_ui;
pub mod model;
pub mod probe;
pub mod state;
pub mod state_audio;
pub mod state_backlight;
pub mod state_bar;
pub mod state_battery;
pub mod state_bluetooth;
pub mod state_date;
pub mod state_dock;
pub mod state_shot;
pub mod state_theme;
pub mod state_wifi;
pub mod util;
pub mod window;
pub mod wire_json;
pub mod wire_vars;

pub use action::Action;
pub use action_file::ActionFile;
pub use action_probe::ActionProbe;
pub use action_shell::ActionShell;
pub use config::Config;
pub use ctx::Req;
pub use ctx::StateId;
pub use event::Event;
pub use event_clock::EventClock;
pub use event_eww::EventEww;
pub use event_job::EventJob;
pub use event_probe::EventProbe;
pub use event_ui::EventUi;
pub use model::Audio;
pub use model::Battery;
pub use model::BtDevice;
pub use model::BtSnapshot;
pub use model::DockApp;
pub use model::MenuId;
pub use model::NetSnapshot;
pub use model::Shot;
pub use model::ShotAction;
pub use model::Theme;
pub use model::WifiNetwork;
pub use probe::JobId;
pub use probe::Probe;
pub use state::PollPlan;
pub use state::State;
pub use state_bar::StateBar;
pub use window::Window;
pub use wire_vars::WireVars;

#[cfg(test)]
pub(crate) mod test_util {
    use std::collections::BTreeMap;
    use std::time::Instant;

    use crate::action::Action;
    use crate::config::Config;
    use crate::ctx::Ctx;
    use crate::ctx::Req;
    use crate::ctx::StateId;
    use crate::state::State;

    /// Drive one sub-machine in isolation and collect what it asked for.
    pub fn run<S: State>(machine: &mut S, event: S::Event) -> Vec<Action> {
        run_full(machine, event).0
    }

    thread_local! {
        /// Job ids never repeat in a running program, so they must not repeat
        /// across the calls a test makes either — two actions sharing one id
        /// would let a stale completion match a fresh job.
        static NEXT_JOB: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    pub fn run_full<S: State>(machine: &mut S, event: S::Event) -> (Vec<Action>, Vec<Req>) {
        let cfg = Config::new("/icons", "/cache");
        let mut next = NEXT_JOB.with(|n| n.get());
        let mut actions = Vec::new();
        let mut owners = BTreeMap::new();
        let mut reqs = Vec::new();
        {
            let mut cx = Ctx::new(
                StateId::Bar,
                Instant::now(),
                &mut next,
                &mut actions,
                &mut owners,
                &mut reqs,
            );
            machine.handle_event(&cfg, event, &mut cx);
        }
        NEXT_JOB.with(|n| n.set(next));
        (actions, reqs)
    }
}
