//! What the bar asks of eww itself, as opposed to what a module asks of a probe.

use crate::server_id::ServerId;

#[derive(Debug)]
pub enum ActionBar {
    /// Nothing is listening. Start a server; the next observation will say
    /// whether it worked.
    EwwStart,
    /// `server` is holding none of our variables and none of our windows, so
    /// everything on it is closed and published again from scratch.
    ///
    /// Two ways to arrive here, and one remedy. It is not who we were talking
    /// to before — a client that could not connect forked itself into a server
    /// and rebound the socket. Or it is, and it re-read its config: `eww
    /// reload` puts every `defvar` back at its default without the process or
    /// the socket changing at all, which the generation stamp is what sees.
    EwwAdopt { server: ServerId },
}
