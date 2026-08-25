//! Listening to the compositor.
//!
//! Hyprland streams events on a socket; we only care whether the focused
//! workspace has a fullscreen window, so the bar and dock can get out of the
//! way. Reacting to the event beats polling for it: the bar hides in the same
//! frame the window goes fullscreen.

use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use eww_state_core::{Event, Intent, Probe};

use crate::runtime::Msg;

/// Events after which the answer to "is anything fullscreen here?" may differ.
fn is_interesting(line: &str) -> bool {
    let name = line.split_once(">>").map(|(n, _)| n).unwrap_or(line);
    matches!(
        name,
        "fullscreen"
            | "workspace"
            | "workspacev2"
            | "focusedmon"
            | "closewindow"
            | "openwindow"
            | "movewindow"
            | "movewindowv2"
            | "changefloatingmode"
    )
}

fn socket_path() -> Option<PathBuf> {
    let run = std::env::var_os("XDG_RUNTIME_DIR")?;
    let sig = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(
        PathBuf::from(run)
            .join("hypr")
            .join(sig)
            .join(".socket2.sock"),
    )
}

/// Follow the compositor's event stream, asking for a fullscreen re-check
/// whenever something happened that could change the answer. Reconnects on its
/// own, so a compositor restart doesn't need one here.
pub fn watch(events: Sender<Msg>) {
    std::thread::Builder::new()
        .name("hypr".into())
        .spawn(move || loop {
            let Some(path) = socket_path() else {
                return; // not running under Hyprland; nothing to watch
            };
            match UnixStream::connect(&path) {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines().map_while(Result::ok) {
                        if is_interesting(&line)
                            && events
                                .send(Msg::Ev(Event::Intent(Intent::Refresh(Probe::Fullscreen))))
                                .is_err()
                        {
                            return;
                        }
                    }
                }
                Err(_) => std::thread::sleep(Duration::from_secs(2)),
            }
            std::thread::sleep(Duration::from_millis(500));
        })
        .expect("spawn hypr watcher");
}

#[cfg(test)]
mod tests {
    use super::is_interesting;

    #[test]
    fn only_layout_changing_events_trigger_a_recheck() {
        assert!(is_interesting("fullscreen>>1"));
        assert!(is_interesting("workspacev2>>2,2"));
        assert!(is_interesting("closewindow>>abc123"));
        assert!(!is_interesting("activelayout>>keyboard,us"));
        assert!(!is_interesting("windowtitle>>abc123"));
    }
}
