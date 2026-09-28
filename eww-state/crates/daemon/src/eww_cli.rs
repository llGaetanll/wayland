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

/// eww's client gives the server 100ms to answer and reads anything slower as
/// there being no server at all: it unlinks the socket, forks itself into a
/// second server, and opens the windows there — leaving the first alive with
/// its bar and dock on screen and nothing able to reach them. Opening the real
/// bar and dock takes longer than 100ms often enough that four servers stacked
/// up in one session. This is the flag that turns that fork back into an error.
const NO_FORK: &str = "--no-daemonize";

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
    ///
    /// The one command that must not carry [`NO_FORK`]: it asks `eww daemon`
    /// to stay in the foreground, and nothing here would ever return.
    pub fn start(&self) -> bool {
        self.exec(&["daemon"])
    }

    /// Whether the server has all of these windows on screen.
    ///
    /// Asked because a command can time out after it worked: the server
    /// answers late rather than wrongly, and the windows are up either way.
    fn are_open(&self, windows: &[String]) -> bool {
        let Some(listed) = self.query(&["active-windows"]) else {
            return false;
        };
        // `<window_id>: <window_name>`, and we open every window under its own
        // name, so the id is the name.
        let up: Vec<&str> = listed
            .lines()
            .filter_map(|l| l.split_once(':'))
            .map(|(_, name)| name.trim())
            .collect();
        windows.iter().all(|w| up.contains(&w.as_str()))
    }

    /// Ask the server something, and say what it said.
    fn query(&self, args: &[&str]) -> Option<String> {
        if self.probe().is_none() {
            return None;
        }
        let mut all = vec!["-c", self.config_dir.to_str().unwrap_or("."), NO_FORK];
        all.extend_from_slice(args);
        exec::out("eww", &all)
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
    /// [`NO_FORK`] is what stops a command that could not be delivered from
    /// becoming a second server. The probe is still worth asking first: it
    /// keeps us from spawning a process at all when there is nothing to talk
    /// to, and it is what tells the machine to start one.
    fn run(&self, args: &[&str]) -> bool {
        if self.probe().is_none() {
            tracing::warn!(?args, "no eww server; command not sent");
            return false;
        }
        let mut all = vec![NO_FORK];
        all.extend_from_slice(args);
        self.exec(&all)
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
        // A slow open is still an open. Taking the exit status for the answer
        // would have us open the same window twice, and eww does not
        // deduplicate that — the second surface is one nothing can close.
        self.run(&args) || self.are_open(windows)
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
