//! Which eww server we are talking to.

/// One eww server, named by the inode of the socket it listens on.
///
/// Not a pid, because a pid is not what we can see. eww's client forks itself
/// into a server whenever it cannot connect, and that new server unlinks the
/// socket path and binds a fresh one — so the inode at that path changing is
/// exactly the handover we have to notice, and it can be read without asking
/// any server anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerId(u64);

impl ServerId {
    pub fn new(inode: u64) -> ServerId {
        ServerId(inode)
    }
}
