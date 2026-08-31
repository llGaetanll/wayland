//! Asking the compositor what its layout is.
//!
//! Two commands, because one of them cannot answer the other's question:
//! `clients` lists every window, and `activeworkspace` says which workspace is
//! focused and which window on it has focus.

use api::JobId;

use crate::event::EventSpaces;
use crate::model::Client;
use crate::model::Snapshot;
use crate::model::WindowAddr;
use crate::model::WorkspaceId;

/// A look produces two events: what it found, and the job finishing. A failed
/// look produces only the second, so nothing waits on it forever.
pub fn look(id: JobId) -> Vec<EventSpaces> {
    let clients = exec::out("hyprctl", &["clients"]);
    let active = exec::out("hyprctl", &["activeworkspace"]);
    let Some((clients, active)) = clients.zip(active) else {
        return vec![EventSpaces::Finished { id }];
    };
    let (focused, focused_window) = parse_active(&active);
    let snapshot = Snapshot {
        focused,
        active: focused_window,
        clients: parse_clients(&clients),
    };
    vec![
        EventSpaces::Sampled { id, snapshot },
        EventSpaces::Finished { id },
    ]
}

/// `hyprctl clients` prints a block per window: a header naming the address,
/// then indented `key: value` lines.
fn parse_clients(out: &str) -> Vec<Client> {
    let mut clients = Vec::new();
    let mut current: Option<Client> = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("Window ") {
            clients.extend(current.take());
            let addr = rest.split_whitespace().next().and_then(WindowAddr::parse);
            current = addr.map(|addr| Client {
                addr,
                workspace: WorkspaceId::default(),
                fullscreen: false,
                monitor: 0,
            });
            continue;
        }
        let Some(client) = current.as_mut() else {
            continue;
        };
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        let value = value.trim();
        // `workspace: 2 (name)` — the id is the first field, the name is the
        // user's and we never interpret it.
        let first = value.split_whitespace().next().unwrap_or(value);
        match key {
            "workspace" => {
                if let Ok(id) = first.parse() {
                    client.workspace = WorkspaceId(id);
                }
            }
            // A mode, not a flag: 0 is neither maximized nor fullscreen, and
            // every other value is the user asking to fill the screen.
            "fullscreen" => client.fullscreen = first != "0",
            "monitor" => client.monitor = first.parse().unwrap_or(0),
            _ => {}
        }
    }
    clients.extend(current);
    clients
}

/// `hyprctl activeworkspace` names the focused workspace and its last focused
/// window — which, on the focused workspace, is the focused window.
fn parse_active(out: &str) -> (WorkspaceId, Option<WindowAddr>) {
    let mut focused = WorkspaceId::default();
    let mut window = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("workspace ID ") {
            if let Ok(id) = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .parse::<i32>()
            {
                focused = WorkspaceId(id);
            }
        }
        if let Some(rest) = line.trim().strip_prefix("lastwindow:") {
            window = WindowAddr::parse(rest);
        }
    }
    (focused, window)
}

#[cfg(test)]
mod tests {
    use super::parse_active;
    use super::parse_clients;

    /// Real `hyprctl clients` output, trimmed to the fields anything reads.
    const CLIENTS: &str = "\
Window 558156ab0980 -> 1:eww-state:
\tat: 0,26
\tworkspace: 2 (2)
\tmonitor: 0
\tclass: Alacritty
\tfullscreen: 1
\tfullscreenClient: 1

Window 5581569f1230 -> Firefox:
\tworkspace: 1 (1)
\tmonitor: 0
\tclass: firefox
\tfullscreen: 0
";

    #[test]
    fn every_window_and_where_it_is() {
        let clients = parse_clients(CLIENTS);
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].addr.as_str(), "0x558156ab0980");
        assert_eq!(clients[0].workspace.0, 2);
        assert!(clients[0].fullscreen);
        assert_eq!(clients[1].workspace.0, 1);
        assert!(
            !clients[1].fullscreen,
            "an ordinary window is not in a Space"
        );
    }

    /// Maximized and fullscreen are different modes and both mean the window
    /// is filling the screen. Only zero is neither.
    #[test]
    fn any_fullscreen_mode_counts() {
        let of = |mode: &str| {
            let out = format!("Window abc -> t:\n\tworkspace: 1 (1)\n\tfullscreen: {mode}\n");
            parse_clients(&out).first().is_some_and(|c| c.fullscreen)
        };
        assert!(!of("0"));
        assert!(of("1"));
        assert!(of("2"));
    }

    #[test]
    fn nothing_readable_is_no_windows() {
        assert!(parse_clients("").is_empty());
        assert!(parse_clients("garbage\n\tfullscreen: 1\n").is_empty());
    }

    #[test]
    fn the_focused_workspace_and_the_window_on_it() {
        let out = "\
workspace ID 2 (2) on monitor eDP-1:
\tmonitorID: 0
\twindows: 1
\thasfullscreen: 1
\tlastwindow: 0x558156ab0980
\tlastwindowtitle: 1:eww-state
";
        let (focused, window) = parse_active(out);
        assert_eq!(focused.0, 2);
        assert_eq!(
            window.map(|w| w.to_string()).as_deref(),
            Some("0x558156ab0980")
        );
    }

    #[test]
    fn an_empty_workspace_has_no_focused_window() {
        let (focused, window) =
            parse_active("workspace ID 7 (7) on monitor eDP-1:\n\tlastwindow: 0x0\n");
        assert_eq!(focused.0, 7);
        assert_eq!(window, None);
    }
}
