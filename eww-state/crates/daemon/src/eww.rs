//! The real eww CLI behind [`EwwSink`].

use std::path::PathBuf;

use crate::proc;
use crate::reconcile::EwwSink;

pub struct EwwCli {
    config_dir: PathBuf,
}

impl EwwCli {
    pub fn new(config_dir: PathBuf) -> Self {
        EwwCli { config_dir }
    }

    fn run(&self, args: &[&str]) -> bool {
        let mut all = vec!["-c", self.config_dir.to_str().unwrap_or(".")];
        all.extend_from_slice(args);
        proc::ok("eww", &all)
    }

    /// Is the eww daemon reachable?
    pub fn alive(&self) -> bool {
        self.run(&["ping"])
    }

    /// Start the daemon if it isn't up. Safe to call repeatedly.
    pub fn ensure_daemon(&self) -> bool {
        self.alive() || self.run(&["daemon"])
    }
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
