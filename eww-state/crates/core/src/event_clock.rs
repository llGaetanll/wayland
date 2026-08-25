//! Time, arriving the same way everything else does.
//!
//! The runtime reads the system clock once a tick and sends it in. Nothing in
//! the machine reads a clock; it answers *may I do this yet* from the last tick
//! it was handed, which is what makes a recorded session replay identically.

use std::time::Instant;

#[derive(Debug)]
pub enum EventClock {
    Tick { now: Instant },
}
