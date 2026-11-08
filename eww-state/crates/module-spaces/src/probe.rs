//! Asking the compositor what its layout is.
//!
//! Two commands, because one of them cannot answer the other's question:
//! `clients` lists every window, and `activeworkspace` says which workspace is
//! focused and which window on it has focus.

use api::JobId;

use crate::event::EventSpaces;
use crate::model::Client;
use crate::model::Rect;
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
                workspace_name: String::new(),
                fullscreen: false,
                floating: false,
                monitor: 0,
                geometry: Rect::default(),
                class: String::new(),
                title: String::new(),
                pid: 0,
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
        // `workspace: 2 (name)` — the id is the first field, the name is in
        // the brackets after it.
        let first = value.split_whitespace().next().unwrap_or(value);
        match key {
            "workspace" => {
                if let Ok(id) = first.parse() {
                    client.workspace = WorkspaceId(id);
                }
                client.workspace_name = bracketed(value).to_string();
            }
            // A mode, not a flag: 0 is neither maximized nor fullscreen, and
            // every other value is the user asking to fill the screen.
            "fullscreen" => client.fullscreen = first != "0",
            "floating" => client.floating = first != "0",
            "monitor" => client.monitor = first.parse().unwrap_or(0),
            "at" => (client.geometry.x, client.geometry.y) = pair(value),
            "size" => (client.geometry.width, client.geometry.height) = pair(value),
            "class" => client.class = value.to_string(),
            "pid" => client.pid = first.parse().unwrap_or(0),
            // Matched on the key rather than taken from the `Window … -> title`
            // header, which truncates and has no delimiter a title cannot
            // contain. `initialTitle` is a different key and does not land here.
            "title" => client.title = value.to_string(),
            _ => {}
        }
    }
    clients.extend(current);
    clients
}

/// The contents of the last pair of brackets on a line, or nothing.
fn bracketed(value: &str) -> &str {
    value
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map(|(inside, _)| inside)
        .unwrap_or_default()
}

/// A `x,y` pair of coordinates. Anything else is the origin, which for a window
/// the compositor has not placed yet is also the truth.
fn pair(value: &str) -> (i32, i32) {
    let Some((a, b)) = value.split_once(',') else {
        return (0, 0);
    };
    (a.trim().parse().unwrap_or(0), b.trim().parse().unwrap_or(0))
}

/// `hyprctl activeworkspace` names the focused workspace and its last focused
/// window — which, on the focused workspace, is the focused window.
fn parse_active(out: &str) -> (WorkspaceId, Option<WindowAddr>) {
    let mut focused = WorkspaceId::default();
    let mut window = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("workspace ") {
            // Hyprland has spelled this header both `workspace ID 2 (2)` and
            // `workspace 2 (2)`, so the word is skipped if it is there rather
            // than required. Getting this wrong leaves no workspace focused,
            // and every rule asking what is on the focused one answers no.
            let rest = rest.strip_prefix("ID ").unwrap_or(rest);
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
\tsize: 1920,1174
\tworkspace: 2 (2)
\tmonitor: 0
\tclass: Alacritty
\ttitle: 1:eww-state
\tfloating: 0
\tfullscreen: 1
\tfullscreenClient: 1

Window 5581569f1230 -> Firefox:
\tat: 210,125
\tsize: 1500,950
\tworkspace: -98 (special:minimized)
\tmonitor: 0
\tclass: firefox
\ttitle: Mozilla Firefox
\tfloating: 1
\tfullscreen: 0
";

    #[test]
    fn every_window_and_where_it_is() {
        let clients = parse_clients(CLIENTS);
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].addr.as_str(), "0x558156ab0980");
        assert_eq!(clients[0].workspace.0, 2);
        assert!(clients[0].fullscreen);
        assert_eq!(clients[0].geometry.width, 1920);
        assert_eq!(clients[1].workspace.0, -98);
        assert!(
            !clients[1].fullscreen,
            "an ordinary window is not in a Space"
        );
    }

    /// Which windows are tiled into the layout and which float over it.
    #[test]
    fn a_window_says_whether_it_floats() {
        let clients = parse_clients(CLIENTS);
        assert!(!clients[0].floating, "tiled into the layout");
        assert!(clients[1].floating);
    }

    /// A special workspace is recognised by name, because its id is whatever
    /// the compositor felt like assigning.
    #[test]
    fn a_workspace_says_what_it_is_called() {
        let clients = parse_clients(CLIENTS);
        assert_eq!(clients[0].workspace_name, "2");
        assert_eq!(clients[1].workspace_name, "special:minimized");
    }

    /// Everything a thumbnail is cropped to, and the title the tile shows.
    #[test]
    fn a_window_says_where_it_is_and_what_it_is() {
        let firefox = &parse_clients(CLIENTS)[1];
        assert_eq!(firefox.geometry.x, 210);
        assert_eq!(firefox.geometry.y, 125);
        assert_eq!(firefox.geometry.width, 1500);
        assert_eq!(firefox.geometry.height, 950);
        assert_eq!(firefox.class, "firefox");
        assert_eq!(firefox.title, "Mozilla Firefox", "not the header's copy");
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

    /// The header has been spelled both ways across Hyprland versions.
    #[test]
    fn the_header_may_omit_the_word_id() {
        let out = "\
workspace 2 (2) on monitor eDP-1:
\tlastwindow: 0x558156ab0980
";
        let (focused, window) = parse_active(out);
        assert_eq!(focused.0, 2);
        assert!(window.is_some());
    }

    #[test]
    fn an_empty_workspace_has_no_focused_window() {
        let (focused, window) =
            parse_active("workspace ID 7 (7) on monitor eDP-1:\n\tlastwindow: 0x0\n");
        assert_eq!(focused.0, 7);
        assert_eq!(window, None);
    }
}
