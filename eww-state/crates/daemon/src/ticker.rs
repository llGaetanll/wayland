//! Interval scheduling for probes.
//!
//! Each probe gets one thread that sleeps until its next sample is due. The
//! interval is not fixed: the state machine recomputes it after every event
//! (an open menu wants fresher data than a bar icon), and a change takes effect
//! immediately rather than after the current sleep runs out.

use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use eww_state_core::{Event, Intent, Probe};

use crate::runtime::Msg;

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
        let mut s = self.state.lock().expect("ticker mutex");
        if s.interval_ms != ms {
            s.interval_ms = ms;
            self.cv.notify_all();
        }
    }

    fn poke(&self) {
        let mut s = self.state.lock().expect("ticker mutex");
        s.pokes += 1;
        self.cv.notify_all();
    }

    /// Block until the next sample is due.
    ///
    /// An interval of 0 means "never on a timer" — the probe then only runs
    /// when something asks for it. Ticker threads live as long as the daemon;
    /// they end with the process.
    fn wait(&self) {
        let start = Instant::now();
        let mut s = self.state.lock().expect("ticker mutex");
        let pokes = s.pokes;
        loop {
            if s.pokes != pokes {
                return;
            }
            let interval = s.interval_ms;
            if interval == 0 {
                s = self.cv.wait(s).expect("ticker mutex");
                continue;
            }
            let due = Duration::from_millis(interval as u64);
            let elapsed = start.elapsed();
            if elapsed >= due {
                return;
            }
            let (guard, _) = self
                .cv
                .wait_timeout(s, due - elapsed)
                .expect("ticker mutex");
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
            std::thread::Builder::new()
                .name(format!("tick-{}", probe.as_str()))
                .spawn(move || loop {
                    ticker.wait();
                    if events
                        .send(Msg::Ev(Event::Intent(Intent::Refresh(probe))))
                        .is_err()
                    {
                        break;
                    }
                })
                .expect("spawn ticker thread");
        }
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
