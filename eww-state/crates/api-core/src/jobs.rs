//! Handing out job ids, and what one is for.
//!
//! A module needs the id of what it asked for so it can recognise the answer:
//! the completion of the join it is still waiting on, as opposed to one from a
//! join it gave up on ten seconds ago. Only the actions that are waited on
//! carry one — a rescan nobody follows up needs no name.
//!
//! Ids are compared only inside the module that issued them, so each module
//! counts on its own and two modules both handing out `JobId(1)` never meet.

/// Handle for a dispatched side effect. It goes out on the action and comes
/// back on the event, which is what stops an answer being attributed to the
/// wrong question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

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
