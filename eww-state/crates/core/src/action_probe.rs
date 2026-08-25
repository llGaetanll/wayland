//! Sampling the world.

use crate::probe::Probe;

#[derive(Debug)]
pub enum ActionProbe {
    /// Sample this now, whatever its interval says.
    Sample { what: Probe },
}
