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

use api::WireVars;
use module_bar::Action;
use module_bar::ActionBar;
use module_bar::ConfigBar;
use module_bar::Event;
use module_bar::EventBar;
use module_bar::ModuleId;
use module_bar::StateBar;

use crate::eww_cli::EwwCli;
use crate::hypr;
use crate::ipc;
use crate::ipc::Query;
use crate::paths::Paths;
use crate::reconcile::Reconciler;
use crate::runtime_perform;
use crate::runtime_perform::Places;
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

const EWW_PROBE_INTERVAL: Duration = Duration::from_secs(5);

pub struct RuntimeDaemon {
    places: Places,
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
        let mut state = StateBar::new(ConfigBar::new(
            &Paths::str(&paths.icons),
            &Paths::str(&paths.cache),
            &Paths::str(&paths.thumbs),
        ));
        let runtime = Arc::new(RuntimeDaemon {
            places: Places::new(
                &paths,
                crate::accent_hex(),
                state.date().format().to_string(),
            ),
            events: adapt(tx.clone()),
        });
        probe_eww(&paths, tx.clone());

        // No bootstrap here any more: starting a server and adopting it are the
        // same two decisions as recovering from one being replaced, and they
        // are the machine's to make. The first observation arrives before the
        // probe's first sleep, so startup takes the ordinary path.
        let mut eww = EwwCli::new(paths.eww_config.clone(), paths.run_dir.clone());
        let mut reconciler = Reconciler::new();

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

                // Every event lands before the machine is asked for anything:
                // a batch is one situation, and what it adds up to is only
                // knowable once all of it has been seen.
                state.handle_events(&events);
                let mut actions = Vec::new();
                state.emit_actions(&mut actions);
                log_turn(turn, &events, &actions);

                // The bar's own actions are carried out here rather than on a
                // thread with the rest: they are about eww and the reconciler,
                // and both live on this one.
                for action in &actions {
                    if let Action::Bar(a) = action {
                        perform_action_bar(a, &mut eww, &mut reconciler);
                    }
                }

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

    /// Dispatch only. Nothing here decides anything: the module that owns the
    /// action carries it out, and whatever it has to say re-enters as an event.
    fn perform_action(&self, action: Action) {
        for event in runtime_perform::perform(&action, &self.places) {
            self.send(event);
        }
    }

    fn send(&self, event: Event) {
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
    let quiet = actions.is_empty() && events.iter().all(|e| matches!(e, Event::Clock(_)));
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

/// Carry out what the bar asked of eww.
///
/// Both of these are recovery, and neither decides anything: whether a server
/// has to be started, and whether the one answering is a stranger, were settled
/// by the machine before this was called.
fn perform_action_bar(action: &ActionBar, eww: &mut EwwCli, reconciler: &mut Reconciler) {
    match action {
        ActionBar::EwwStart => {
            tracing::warn!("nothing listening at eww's socket; starting a server");
            if !eww.start() {
                tracing::error!("could not start an eww server");
            }
        }
        ActionBar::EwwAdopt { server } => {
            tracing::warn!(?server, "publishing to an eww holding none of our vars");
            // close-all before anything is published: a server we did not open
            // these on may still be holding windows from whoever did.
            reconciler.reset(eww);
        }
    }
}

/// Report what is listening at eww's socket and what it is holding, over and
/// over.
///
/// The thread holds nothing and concludes nothing — it says what it saw and the
/// machine decides what that means. It used to keep a `was_alive` of its own
/// and decide "eww restarted" from it, which could not see the failure that
/// actually happens: a client forking itself into a second server reads as
/// alive both before and after, so three servers accumulated over three days
/// without this ever firing once.
fn probe_eww(paths: &Paths, tx: Sender<Msg>) {
    let eww = EwwCli::new(paths.eww_config.clone(), paths.run_dir.clone());
    let spawned = std::thread::Builder::new()
        .name("eww-probe".into())
        .spawn(move || loop {
            let server = eww.probe();
            // Only worth asking when something is listening, and asking costs
            // a process: the stamp is read at the probe's pace, not the loop's.
            let gen = server.and_then(|_| eww.read_gen());
            if tx
                .send(Msg::Ev(Event::Bar(EventBar::EwwObserved { server, gen })))
                .is_err()
            {
                return;
            }
            std::thread::sleep(EWW_PROBE_INTERVAL);
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not probe eww");
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

fn render_polls(plan: &BTreeMap<ModuleId, u32>) -> String {
    ModuleId::ALL
        .iter()
        .map(|id| match plan.get(id) {
            Some(ms) => format!("{}: {}ms", id.as_str(), ms),
            None => format!("{}: on demand", id.as_str()),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
