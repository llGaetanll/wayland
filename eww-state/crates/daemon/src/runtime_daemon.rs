//! The daemon: one event loop, everything else feeding it.
//!
//! Exactly one thread ever touches the state machine, and it does nothing slow.
//! Probes, actions, the compositor listener and the control socket all run
//! elsewhere and communicate only by sending events. That is what makes the
//! ordering deterministic: no two clicks, and no click and poll, can interleave
//! half-way through each other's work.
//!
//! One turn is: collect the events waiting, hand them to the machine, carry out
//! the actions, log both. Whatever the IO produces re-enters as the next event,
//! which closes the loop.

use std::collections::BTreeMap;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use eww_state_core::Action;
use eww_state_core::Config;
use eww_state_core::Event;
use eww_state_core::EventEww;
use eww_state_core::Probe;
use eww_state_core::StateBar;
use eww_state_core::WireVars;

use crate::eww_cli::EwwCli;
use crate::hypr;
use crate::ipc;
use crate::ipc::Query;
use crate::paths::Paths;
use crate::reconcile::Reconciler;
use crate::ticker::Scheduler;

/// Anything the event loop can receive.
pub enum Msg {
    Ev(Event),
    /// A read-only question from the CLI, answered on `reply`.
    Query {
        kind: Query,
        reply: Sender<String>,
    },
}

const EWW_HEALTH_INTERVAL: Duration = Duration::from_secs(5);

pub struct RuntimeDaemon {
    paths: Paths,
    events: Sender<Event>,
}

impl RuntimeDaemon {
    /// Start the daemon and run until the last sender goes away.
    ///
    /// The only public function here. Everything else is reachable solely by
    /// carrying out an action, so nothing can do IO without one asking for it.
    pub fn run(paths: Paths) -> std::io::Result<()> {
        paths.ensure_dirs();
        let listener = ipc::bind(&paths)?;

        let (tx, rx) = channel::<Msg>();
        ipc::serve(listener, tx.clone());
        hypr::watch(tx.clone());
        let scheduler = Scheduler::start(tx.clone());

        // Actions speak plain events; adapt them onto the loop's channel here
        // so nothing outside this module needs to know about `Msg`.
        let runtime = Arc::new(RuntimeDaemon {
            paths: paths.clone(),
            events: adapt(tx.clone()),
        });
        watch_eww(&paths, tx.clone());

        let mut eww = EwwCli::new(paths.eww_config.clone());
        eww.ensure_daemon();
        let mut reconciler = Reconciler::new();
        reconciler.reset(&mut eww);

        let mut state = StateBar::new(
            Config::new(Paths::str(&paths.icons), Paths::str(&paths.cache)),
            std::time::Instant::now(),
        );

        tracing::info!(commit = crate::log::COMMIT, "daemon started");
        // Populate everything before the first interval elapses.
        scheduler.apply(&state.polls());
        scheduler.poke_all();

        let mut turn = 0u64;
        loop {
            let Ok(first) = rx.recv() else { return Ok(()) };
            let (events, query) = collect(first, &rx);

            if !events.is_empty() {
                turn += 1;

                if events
                    .iter()
                    .any(|e| matches!(e, Event::FromEww(EventEww::Restarted)))
                {
                    tracing::warn!("eww went away; republishing");
                    eww.ensure_daemon();
                    reconciler.forget();
                }

                let mut actions = Vec::new();
                state.handle_events(&events, &mut actions);
                log_turn(turn, &events, &actions);

                // Content first, then windows: a panel never appears showing
                // what it looked like last time it was open.
                reconciler.sync_vars(&state.vars(), &mut eww);
                reconciler.sync_windows(&state.windows(), &mut eww);
                scheduler.apply(&state.polls());

                runtime.perform_actions(actions);
            }

            if let Some((kind, reply)) = query {
                let _ = reply.send(answer(&state, kind));
            }
        }
    }

    /// Each action gets its own thread: a ten-second `bluetoothctl connect` or
    /// a screenshot the user never finishes selecting can't hold up anything
    /// else, and the machine is not waiting on any of them.
    fn perform_actions(self: &Arc<Self>, actions: Vec<Action>) {
        for action in actions {
            let runtime = Arc::clone(self);
            let spawned = std::thread::Builder::new()
                .name("action".into())
                .spawn(move || runtime.perform_action(action));
            if let Err(e) = spawned {
                tracing::error!(error = %e, "could not spawn an action thread");
            }
        }
    }

    /// Dispatch only. Nothing here decides anything: the outcome goes back to
    /// the machine, which owns what it means.
    fn perform_action(&self, action: Action) {
        let id = action.id();
        let (ok, detail) = match action {
            Action::ToProbe { action, .. } => self.perform_action_probe(action),
            Action::ToShell { action, .. } => self.perform_action_shell(action),
            Action::ToFile { action, .. } => self.perform_action_file(action),
        };
        self.send(Event::FromJob(eww_state_core::EventJob::Finished {
            id,
            ok,
            detail,
        }));
    }

    pub(crate) fn paths(&self) -> &Paths {
        &self.paths
    }

    pub(crate) fn send(&self, event: Event) {
        if self.events.send(event).is_err() {
            tracing::debug!("the loop has gone; dropping an event");
        }
    }
}

/// Write the turn down: what went in, and what came back out.
///
/// A turn that is nothing but clock ticks and produced nothing is logged a
/// level down. Two lines a second of "the time passed and nothing happened"
/// would otherwise roll the whole file over every ninety minutes, and the
/// history a bug report needs would be gone. `EWW_STATE_LOG=debug` brings them
/// back.
fn log_turn(turn: u64, events: &[Event], actions: &[Action]) {
    let quiet = actions.is_empty() && events.iter().all(|e| matches!(e, Event::FromClock(_)));
    if quiet {
        tracing::debug!(turn, n = events.len(), events = ?events, "state -> events");
        tracing::debug!(turn, n = 0, "state <- actions");
        return;
    }
    tracing::info!(turn, n = events.len(), events = ?events, "state -> events");
    tracing::info!(turn, n = actions.len(), actions = ?actions, "state <- actions");
}

/// Take everything already waiting, so one turn is one batch. Draining stops at
/// a query: it must be answered against the state the events before it left.
fn collect(first: Msg, rx: &std::sync::mpsc::Receiver<Msg>) -> (Vec<Event>, Option<Answer>) {
    let mut events = Vec::new();
    let mut query = None;
    let mut take = |msg| match msg {
        Msg::Ev(e) => {
            events.push(e);
            true
        }
        Msg::Query { kind, reply } => {
            query = Some((kind, reply));
            false
        }
    };
    if take(first) {
        while let Ok(msg) = rx.try_recv() {
            if !take(msg) {
                break;
            }
        }
    }
    (events, query)
}

type Answer = (Query, Sender<String>);

/// Bridge the plain `Event` channel actions speak onto the loop's `Msg` one.
fn adapt(tx: Sender<Msg>) -> Sender<Event> {
    let (ev_tx, ev_rx) = channel::<Event>();
    let spawned = std::thread::Builder::new()
        .name("events".into())
        .spawn(move || {
            for event in ev_rx {
                if tx.send(Msg::Ev(event)).is_err() {
                    return;
                }
            }
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not spawn the event adapter");
    }
    ev_tx
}

/// Notice an eww daemon that died or was reloaded, so its state can be pushed
/// again. Without this the bar would come back empty and stay that way.
fn watch_eww(paths: &Paths, tx: Sender<Msg>) {
    let eww = EwwCli::new(paths.eww_config.clone());
    let spawned = std::thread::Builder::new()
        .name("eww-health".into())
        .spawn(move || {
            let mut was_alive = true;
            loop {
                std::thread::sleep(EWW_HEALTH_INTERVAL);
                let alive = eww.alive();
                // Only on the transition: a dead eww shouldn't produce a reset
                // event every five seconds.
                let restarted = Event::FromEww(EventEww::Restarted);
                if alive && !was_alive && tx.send(Msg::Ev(restarted)).is_err() {
                    return;
                }
                was_alive = alive;
            }
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not watch eww's health");
    }
}

fn answer(state: &StateBar, kind: Query) -> String {
    match kind {
        Query::Vars => render_vars(&state.vars()),
        Query::Windows => render_windows(state),
        Query::Polls => render_polls(&state.polls()),
        Query::Dump => format!(
            "menu: {}\n\n[vars]\n{}\n[windows]\n{}\n\n[polls]\n{}",
            state.menu().map(|m| m.as_str()).unwrap_or("-"),
            render_vars(&state.vars()),
            render_windows(state),
            render_polls(&state.polls()),
        ),
    }
}

fn render_vars(vars: &WireVars) -> String {
    vars.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_windows(state: &StateBar) -> String {
    state
        .windows()
        .into_iter()
        .map(|w| w.name())
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_polls(plan: &BTreeMap<Probe, u32>) -> String {
    plan.iter()
        .map(|(p, ms)| {
            if *ms == 0 {
                format!("{}: on demand", p.as_str())
            } else {
                format!("{}: {}ms", p.as_str(), ms)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
