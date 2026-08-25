//! The daemon: one event loop, everything else feeding it.
//!
//! Exactly one thread ever touches the state machine, and it does nothing slow.
//! Probes, actions, the compositor listener and the control socket all run
//! elsewhere and communicate only by sending events. That is what makes the
//! ordering deterministic: no two clicks, and no click and poll, can interleave
//! half-way through each other's work.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

use eww_state_core::{Bar, Config, Event, Probe, Vars};

use crate::act::Runner;
use crate::eww::EwwCli;
use crate::hypr;
use crate::ipc::{self, Cmd, Query};
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

pub fn run(paths: Paths) -> std::io::Result<()> {
    paths.ensure_dirs();
    let listener = bind_socket(&paths)?;

    let (tx, rx) = channel::<Msg>();
    serve(listener, tx.clone());
    hypr::watch(tx.clone());
    let scheduler = Scheduler::start(tx.clone());

    // Actions speak plain events; adapt them onto the loop's channel here so
    // nothing outside this module needs to know about `Msg`.
    let runner = {
        let (ev_tx, ev_rx) = channel::<Event>();
        let tx = tx.clone();
        std::thread::Builder::new()
            .name("events".into())
            .spawn(move || {
                for ev in ev_rx {
                    if tx.send(Msg::Ev(ev)).is_err() {
                        break;
                    }
                }
            })
            .expect("spawn event adapter");
        Arc::new(Runner {
            paths: paths.clone(),
            events: ev_tx,
        })
    };

    watch_eww(&paths, tx.clone());

    let mut eww = EwwCli::new(paths.eww_config.clone());
    eww.ensure_daemon();
    let mut rec = Reconciler::new();
    rec.reset(&mut eww);

    let mut bar = Bar::new(Config::new(
        Paths::str(&paths.icons),
        Paths::str(&paths.cache),
    ));

    log(&paths, "daemon started");
    // Populate everything before the first interval elapses.
    scheduler.apply(&bar.polls());
    scheduler.poke_all();

    for msg in rx {
        match msg {
            Msg::Ev(ev) => {
                if matches!(ev, Event::EwwReset) {
                    log(&paths, "eww went away; republishing");
                    eww.ensure_daemon();
                    rec.forget();
                }
                let actions = bar.handle(ev);

                // Content first, then windows: a panel never appears showing
                // what it looked like last time it was open.
                rec.sync_vars(&bar.vars(), &mut eww);
                rec.sync_windows(&bar.windows(), &mut eww);
                scheduler.apply(&bar.polls());

                for action in actions {
                    let runner = Arc::clone(&runner);
                    std::thread::Builder::new()
                        .name("action".into())
                        .spawn(move || runner.run(action))
                        .expect("spawn action thread");
                }
            }
            Msg::Query { kind, reply } => {
                let _ = reply.send(answer(&bar, kind));
            }
        }
    }
    Ok(())
}

/// Refuse to start a second daemon, but clean up after one that died.
fn bind_socket(paths: &Paths) -> std::io::Result<UnixListener> {
    if paths.socket.exists() {
        if ipc::request(&paths.socket, &["ping".to_string()]).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "eww-state is already running",
            ));
        }
        let _ = std::fs::remove_file(&paths.socket);
    }
    UnixListener::bind(&paths.socket)
}

fn serve(listener: UnixListener, tx: Sender<Msg>) {
    std::thread::Builder::new()
        .name("ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let tx = tx.clone();
                // A connection per click, each handled on its own thread: a
                // client that stops reading can't stall anyone else.
                let _ = std::thread::Builder::new()
                    .name("ipc-conn".into())
                    .spawn(move || handle_conn(stream, tx));
            }
        })
        .expect("spawn ipc listener");
}

fn handle_conn(stream: UnixStream, tx: Sender<Msg>) {
    let mut line = String::new();
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let args: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
    let mut out = stream;
    let reply = match ipc::parse(&args) {
        Err(e) => format!("error: {e}\n"),
        Ok(Cmd::Ping) => "ok\n".to_string(),
        Ok(Cmd::Ev(ev)) => match tx.send(Msg::Ev(ev)) {
            Ok(()) => "ok\n".to_string(),
            Err(_) => "error: daemon shutting down\n".to_string(),
        },
        Ok(Cmd::Query(kind)) => {
            let (reply_tx, reply_rx) = channel();
            if tx
                .send(Msg::Query {
                    kind,
                    reply: reply_tx,
                })
                .is_err()
            {
                "error: daemon shutting down\n".to_string()
            } else {
                reply_rx
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap_or_else(|_| "error: no answer\n".to_string())
            }
        }
    };
    let _ = out.write_all(reply.as_bytes());
}

/// Notice an eww daemon that died or was reloaded, so its state can be pushed
/// again. Without this the bar would come back empty and stay that way.
fn watch_eww(paths: &Paths, tx: Sender<Msg>) {
    let eww = EwwCli::new(paths.eww_config.clone());
    std::thread::Builder::new()
        .name("eww-health".into())
        .spawn(move || {
            let mut was_alive = true;
            loop {
                std::thread::sleep(EWW_HEALTH_INTERVAL);
                let alive = eww.alive();
                // Only on the transition: a dead eww shouldn't produce a reset
                // event every five seconds.
                if alive && !was_alive && tx.send(Msg::Ev(Event::EwwReset)).is_err() {
                    return;
                }
                was_alive = alive;
            }
        })
        .expect("spawn eww health thread");
}

fn answer(bar: &Bar, kind: Query) -> String {
    match kind {
        Query::Vars => render_vars(&bar.vars()),
        Query::Windows => bar
            .windows()
            .into_iter()
            .map(|w| w.name())
            .collect::<Vec<_>>()
            .join("\n"),
        Query::Polls => render_polls(&bar.polls()),
        Query::Dump => format!(
            "menu: {}\n\n[vars]\n{}\n[windows]\n{}\n\n[polls]\n{}",
            bar.menu().map(|m| m.as_str()).unwrap_or("-"),
            render_vars(&bar.vars()),
            bar.windows()
                .into_iter()
                .map(|w| w.name())
                .collect::<Vec<_>>()
                .join("\n"),
            render_polls(&bar.polls()),
        ),
    }
}

fn render_vars(vars: &Vars) -> String {
    vars.iter()
        .map(|(k, v)| format!("{k}={v}"))
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

fn log(paths: &Paths, msg: &str) {
    use std::fs::OpenOptions;
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.log)
    {
        let stamp = crate::proc::out("date", &["+%F %T"]).unwrap_or_default();
        let _ = writeln!(f, "[{stamp}] {msg}");
    }
}
