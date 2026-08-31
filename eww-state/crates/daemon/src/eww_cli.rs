//! The real eww CLI behind [`EwwSink`].

use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;

use module_bar::ServerId;

use crate::reconcile::EwwSink;

/// eww names its socket after a hash of the config path, and offers no way to
/// ask it which. The runtime directory is scanned for this prefix instead.
const SOCKET_PREFIX: &str = "eww-server_";

pub struct EwwCli {
    config_dir: PathBuf,
    run_dir: PathBuf,
}

impl EwwCli {
    pub fn new(config_dir: PathBuf, run_dir: PathBuf) -> Self {
        EwwCli {
            config_dir,
            run_dir,
        }
    }

    /// Which server is listening at eww's socket, if any.
    ///
    /// We connect ourselves rather than shelling out to `eww ping`, because
    /// every eww client starts a server when it cannot reach one: asking eww
    /// whether a server is running is a way to end up with a server running.
    /// The answer is the socket's inode, which is what changes when a new
    /// server unlinks the path and binds its own.
    pub fn probe(&self) -> Option<ServerId> {
        let path = self.socket()?;
        let inode = std::fs::metadata(&path).ok()?.ino();
        UnixStream::connect(&path).ok()?;
        Some(ServerId::new(inode))
    }

    /// Start a server. Only ever called because the machine asked for one.
    pub fn start(&self) -> bool {
        self.exec(&["daemon"])
    }

    fn socket(&self) -> Option<PathBuf> {
        let mut found: Option<PathBuf> = None;
        for entry in std::fs::read_dir(&self.run_dir).ok()? {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if !is_server_socket(&path) {
                continue;
            }
            if found.is_some() {
                tracing::warn!("more than one eww socket; cannot say which is ours");
                return None;
            }
            found = Some(path);
        }
        found
    }

    /// Send one command to the server, and only if there is a server to send it
    /// to.
    ///
    /// This gate is the point of the probe. An eww client that cannot connect
    /// does not fail — it forks itself into a second server, which unlinks the
    /// socket and binds its own, leaving the first alive with its bar and dock
    /// still on screen and nothing able to reach them. Three of them ended up
    /// stacked that way. The check narrows that to the gap between here and the
    /// client's own connect rather than closing it; what slips through is
    /// caught by the next probe, which sees a server it does not recognise.
    fn run(&self, args: &[&str]) -> bool {
        if self.probe().is_none() {
            tracing::warn!(?args, "no eww server; command not sent");
            return false;
        }
        self.exec(args)
    }

    fn exec(&self, args: &[&str]) -> bool {
        // A non-UTF-8 config path is not something we can hand to a command
        // line; the current directory is what eww would default to anyway.
        let mut all = vec!["-c", self.config_dir.to_str().unwrap_or(".")];
        all.extend_from_slice(args);
        exec::ok("eww", &all)
    }
}

fn is_server_socket(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with(SOCKET_PREFIX))
}

impl EwwSink for EwwCli {
    fn update(&mut self, pairs: &[(&str, &str)]) -> bool {
        let assignments: Vec<String> = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let mut args = vec!["update"];
        args.extend(assignments.iter().map(String::as_str));
        self.run(&args)
    }

    fn open(&mut self, windows: &[String]) -> bool {
        let mut args = vec!["open-many"];
        args.extend(windows.iter().map(String::as_str));
        self.run(&args)
    }

    fn close(&mut self, windows: &[String]) -> bool {
        let mut args = vec!["close"];
        args.extend(windows.iter().map(String::as_str));
        self.run(&args)
    }

    fn close_all(&mut self) -> bool {
        self.run(&["close-all"])
    }
}
