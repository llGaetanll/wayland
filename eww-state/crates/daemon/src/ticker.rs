//! Interval scheduling for probes, and the clock the machine runs on.
//!
//! Each probe gets one thread that sleeps until its next sample is due. The
//! interval is not fixed: the state machine recomputes it after every batch
//! (an open menu wants fresher data than a bar icon), and a change takes effect
//! immediately rather than after the current sleep runs out.
//!
//! One more thread does nothing but read the system clock once a second and
//! send it in. Sleeping is IO, so the machine cannot do it; the tick is how a
//! rule that has to wait gets asked again.

use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

use eww_state_core::Event;
use eww_state_core::EventClock;
use eww_state_core::EventProbe;
use eww_state_core::Probe;

use crate::runtime_daemon::Msg;

/// How often the machine is told what time it is. Coarse on purpose: a rule
/// needing finer resolution than this is a rule about to depend on scheduling
/// luck.
const CLOCK_INTERVAL: Duration = Duration::from_secs(1);

struct TickState {
    interval_ms: u32,
    /// Bumped to demand a sample right now.
    pokes: u64,
}

pub struct Ticker {
    state: Mutex<TickState>,
    cv: Condvar,
}

impl Ticker {
    fn new(interval_ms: u32) -> Ticker {
        Ticker {
            state: Mutex::new(TickState {
                interval_ms,
                pokes: 0,
            }),
            cv: Condvar::new(),
        }
    }

    fn set_interval(&self, ms: u32) {
        let mut s = self.lock();
        if s.interval_ms != ms {
            s.interval_ms = ms;
            self.cv.notify_all();
        }
    }

    fn poke(&self) {
        let mut s = self.lock();
        s.pokes += 1;
        self.cv.notify_all();
    }

    /// A mutex poisoned by another thread's panic still describes a valid
    /// interval, and refusing to tick would be worse than reading it.
    fn lock(&self) -> std::sync::MutexGuard<'_, TickState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Block until the next sample is due.
    ///
    /// An interval of 0 means "never on a timer" — the probe then only runs
    /// when something asks for it. Ticker threads live as long as the daemon;
    /// they end with the process.
    fn wait(&self) {
        let start = Instant::now();
        let mut s = self.lock();
        let pokes = s.pokes;
        loop {
            if s.pokes != pokes {
                return;
            }
            let interval = s.interval_ms;
            if interval == 0 {
                let Ok(guard) = self.cv.wait(s) else { return };
                s = guard;
                continue;
            }
            let due = Duration::from_millis(interval as u64);
            let elapsed = start.elapsed();
            if elapsed >= due {
                return;
            }
            let Ok((guard, _)) = self.cv.wait_timeout(s, due - elapsed) else {
                return;
            };
            s = guard;
        }
    }
}

/// One ticker per probe, kept in step with what the machine currently wants.
pub struct Scheduler {
    tickers: BTreeMap<Probe, Arc<Ticker>>,
}

impl Scheduler {
    /// Start a thread per probe. Each asks the machine for a sample, so probing
    /// stays a normal event round-trip rather than a side channel.
    pub fn start(events: Sender<Msg>) -> Scheduler {
        let mut tickers = BTreeMap::new();
        for probe in Probe::ALL {
            let ticker = Arc::new(Ticker::new(0));
            tickers.insert(probe, Arc::clone(&ticker));
            let events = events.clone();
            spawn(&format!("tick-{}", probe.as_str()), move || loop {
                ticker.wait();
                let due = Event::FromProbe(EventProbe::Due { what: probe });
                if events.send(Msg::Ev(due)).is_err() {
                    return;
                }
            });
        }
        spawn("clock", move || loop {
            std::thread::sleep(CLOCK_INTERVAL);
            let tick = Event::FromClock(EventClock::Tick {
                now: Instant::now(),
            });
            if events.send(Msg::Ev(tick)).is_err() {
                return;
            }
        });
        Scheduler { tickers }
    }

    pub fn apply(&self, plan: &BTreeMap<Probe, u32>) {
        for (probe, ticker) in &self.tickers {
            ticker.set_interval(plan.get(probe).copied().unwrap_or(0));
        }
    }

    /// Sample everything now — used once at startup so the bar is populated
    /// before the first interval elapses.
    pub fn poke_all(&self) {
        for ticker in self.tickers.values() {
            ticker.poke();
        }
    }
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    let spawned = std::thread::Builder::new().name(name.to_string()).spawn(f);
    if let Err(e) = spawned {
        tracing::error!(error = %e, thread = name, "could not spawn a timer thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poke_wakes_a_ticker_that_would_never_fire() {
        let t = Arc::new(Ticker::new(0));
        let t2 = Arc::clone(&t);
        let handle = std::thread::spawn(move || t2.wait());
        // Give the waiter a moment to park, then wake it.
        std::thread::sleep(Duration::from_millis(20));
        t.poke();
        handle.join().expect("the waiter woke up");
    }

    #[test]
    fn shortening_the_interval_takes_effect_immediately() {
        let t = Arc::new(Ticker::new(60_000));
        let t2 = Arc::clone(&t);
        let start = Instant::now();
        let handle = std::thread::spawn(move || t2.wait());
        std::thread::sleep(Duration::from_millis(20));
        t.set_interval(30);
        handle.join().expect("the waiter woke up");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the sleeping ticker picked up the new interval"
        );
    }
}
