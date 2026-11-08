//! Time, arriving the same way everything else does.
//!
//! The runtime reads the system clock once a tick and sends it in. Nothing in
//! any machine reads a clock; each answers *may I do this yet* from the last
//! tick it was handed, which is what makes a recorded session replay
//! identically.

use api::Millis;

#[derive(Debug)]
pub enum EventClock {
    Tick { now: Millis },
}
