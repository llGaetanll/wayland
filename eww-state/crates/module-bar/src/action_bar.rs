//! What the bar asks of eww itself, as opposed to what a module asks of a probe.

use crate::server_id::ServerId;

#[derive(Debug)]
pub enum ActionBar {
    /// Nothing is listening. Start a server; the next observation will say
    /// whether it worked.
    EwwStart,
    /// `server` is who we are talking to now, and it is not who we were talking
    /// to before. It holds none of our variables and none of our windows, so
    /// everything on it is closed and published again from scratch.
    EwwAdopt { server: ServerId },
}
