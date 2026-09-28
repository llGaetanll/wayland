//! Where everything lives. The only place this crate reads the environment.

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Paths {
    /// eww's own config directory (holds eww.yuck).
    pub eww_config: PathBuf,
    pub icons: PathBuf,
    pub cache: PathBuf,
    /// Our control socket; the CLI connects here.
    pub socket: PathBuf,
    /// The runtime directory, which is also where eww puts its own socket.
    pub run_dir: PathBuf,
    /// Dock contents and order, one app per line.
    pub dock_conf: PathBuf,
    /// Staging area for captured screenshots.
    pub shots: PathBuf,
    /// Staging area for the thumbnails of minimized windows.
    pub thumbs: PathBuf,
    /// Where "Save" puts a screenshot.
    pub screenshots: PathBuf,
    /// The rotating log, under the platform's state directory rather than the
    /// cache: a log someone is about to send us is not disposable.
    pub log: PathBuf,
}

fn env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

impl Paths {
    pub fn discover() -> Paths {
        let home = env_path("HOME").unwrap_or_else(|| PathBuf::from("/tmp"));
        let config = env_path("XDG_CONFIG_HOME").unwrap_or_else(|| home.join(".config"));
        let cache = env_path("XDG_CACHE_HOME").unwrap_or_else(|| home.join(".cache"));
        let state = env_path("XDG_STATE_HOME").unwrap_or_else(|| home.join(".local/state"));
        let run = env_path("XDG_RUNTIME_DIR").unwrap_or_else(|| PathBuf::from("/tmp"));

        Paths {
            eww_config: config.join("eww"),
            icons: config.join("eww/icons"),
            cache: cache.join("eww"),
            socket: run.join("eww-state.sock"),
            run_dir: run.clone(),
            dock_conf: config.join("eww-state/dock.conf"),
            shots: run.join("eww-shots"),
            thumbs: run.join("eww-minimized"),
            screenshots: home.join("files/images/screenshots"),
            log: state.join("eww-state/eww-state.log"),
        }
    }

    pub fn ensure_dirs(&self) {
        for d in [&self.cache, &self.shots, &self.screenshots, &self.thumbs] {
            let _ = std::fs::create_dir_all(d);
        }
        // Nothing here outlives the daemon that made it: a thumbnail is only
        // ever shown for a window the running machine remembers minimizing, and
        // it does not remember across a restart.
        reap(&self.thumbs);
        if let Some(parent) = self.dock_conf.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    pub fn str(p: &std::path::Path) -> String {
        p.to_string_lossy().into_owned()
    }
}

/// Empty a staging directory of the files a previous run left in it.
fn reap(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        if e.path().extension().is_some_and(|x| x == "png") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}
