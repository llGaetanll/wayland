//! Talking to the compositor.
//!
//! Hyprland streams events on a socket; we only care whether the focused
//! workspace has a fullscreen window, so the bar and dock can get out of the
//! way. Reacting to the event beats polling for it: the bar hides in the same
//! frame the window goes fullscreen.

use std::io::BufRead;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use eww_state_core::Event;
use eww_state_core::EventProbe;
use eww_state_core::Probe;

use crate::proc;
use crate::runtime_daemon::Msg;

pub fn sample() -> Option<EventProbe> {
    let out = proc::out("hyprctl", &["activeworkspace"])?;
    Some(EventProbe::Fullscreen {
        on: parse_fullscreen(&out),
    })
}

fn parse_fullscreen(out: &str) -> bool {
    out.lines()
        .find_map(|l| l.trim().strip_prefix("hasfullscreen:"))
        .map(|v| v.trim() != "0")
        .unwrap_or(false)
}

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
    let spawned = std::thread::Builder::new()
        .name("hypr".into())
        .spawn(move || loop {
            let Some(path) = socket_path() else {
                tracing::info!("not running under Hyprland; fullscreen is not watched");
                return;
            };
            match UnixStream::connect(&path) {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines().map_while(Result::ok) {
                        if !is_interesting(&line) {
                            continue;
                        }
                        let due = Event::FromProbe(EventProbe::Due {
                            what: Probe::Fullscreen,
                        });
                        if events.send(Msg::Ev(due)).is_err() {
                            return;
                        }
                    }
                }
                Err(_) => std::thread::sleep(Duration::from_secs(2)),
            }
            std::thread::sleep(Duration::from_millis(500));
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not watch the compositor");
    }
}

#[cfg(test)]
mod tests {
    use super::is_interesting;
    use super::parse_fullscreen;

    #[test]
    fn only_layout_changing_events_trigger_a_recheck() {
        assert!(is_interesting("fullscreen>>1"));
        assert!(is_interesting("workspacev2>>2,2"));
        assert!(is_interesting("closewindow>>abc123"));
        assert!(!is_interesting("activelayout>>keyboard,us"));
        assert!(!is_interesting("windowtitle>>abc123"));
    }

    #[test]
    fn fullscreen_flag() {
        assert!(parse_fullscreen(
            "workspace ID 2 (2) on monitor eDP-1:\n\tmonitorID: 0\n\thasfullscreen: 1"
        ));
        assert!(!parse_fullscreen("\thasfullscreen: 0"));
        assert!(!parse_fullscreen(""));
    }
}
