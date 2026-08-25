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
    /// Dock contents and order, one app per line.
    pub dock_conf: PathBuf,
    /// Staging area for captured screenshots.
    pub shots: PathBuf,
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
            dock_conf: config.join("eww-state/dock.conf"),
            shots: run.join("eww-shots"),
            screenshots: home.join("Pictures/Screenshots"),
            log: state.join("eww-state/eww-state.log"),
        }
    }

    pub fn ensure_dirs(&self) {
        for d in [&self.cache, &self.shots, &self.screenshots] {
            let _ = std::fs::create_dir_all(d);
        }
        if let Some(parent) = self.dock_conf.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    pub fn str(p: &std::path::Path) -> String {
        p.to_string_lossy().into_owned()
    }
}
