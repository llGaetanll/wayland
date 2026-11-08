//! Listening to the compositor.
//!
//! Hyprland streams events on a socket. Nothing here reads one of them for its
//! content: the line only says that the layout may have changed, and the Spaces
//! module answers that by looking. Reacting to the event beats polling for it —
//! the bar hides in the same frame a window goes fullscreen.

use std::io::BufRead;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use module_bar::ModuleId;

use crate::runtime_daemon::Msg;

/// Events after which the layout may look different.
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

/// Follow the compositor's event stream, asking for a fresh look whenever
/// something happened that could have changed the layout. Reconnects on its
/// own, so a compositor restart doesn't need one here.
pub fn watch(events: Sender<Msg>) {
    let spawned = std::thread::Builder::new()
        .name("hypr".into())
        .spawn(move || loop {
            let Some(path) = socket_path() else {
                tracing::info!("not running under Hyprland; the layout is not watched");
                return;
            };
            match UnixStream::connect(&path) {
                Ok(stream) => {
                    let reader = BufReader::new(stream);
                    for line in reader.lines().map_while(Result::ok) {
                        if !is_interesting(&line) {
                            continue;
                        }
                        if events.send(Msg::Ev(ModuleId::Spaces.due())).is_err() {
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

    #[test]
    fn only_layout_changing_events_ask_for_a_look() {
        assert!(is_interesting("fullscreen>>1"));
        assert!(is_interesting("workspacev2>>2,2"));
        assert!(is_interesting("closewindow>>abc123"));
        assert!(!is_interesting("activelayout>>keyboard,us"));
        assert!(!is_interesting("windowtitle>>abc123"));
    }
}
