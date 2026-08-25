//! Handing out job ids.
//!
//! A machine needs the id of what it asked for, so it can recognise the answer
//! and ignore one meant for a request it has since abandoned. That is all this
//! is: a counter, passed to whoever is emitting.

use crate::probe::JobId;

#[derive(Clone, Debug, Default)]
pub struct Jobs(u64);

impl Jobs {
    /// The next unused id. Not `next` — clippy reads that as an iterator, and
    /// this hands out one id rather than walking a sequence anyone consumes.
    pub fn take(&mut self) -> JobId {
        self.0 += 1;
        JobId(self.0)
    }
}
