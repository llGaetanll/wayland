//! Carrying out a shell action.
//!
//! Each of these runs a program and reports whether it worked. Nothing here
//! decides anything — where a command needs a second try or a prompt, that is
//! how the command works, not a rule about the widget.

use eww_state_core::ActionShell;
use eww_state_core::Event;
use eww_state_core::EventJob;
use eww_state_core::Shot;

use crate::paths::Paths;
use crate::proc;
use crate::runtime_daemon::RuntimeDaemon;

impl RuntimeDaemon {
    pub(crate) fn perform_action_shell(&self, action: ActionShell) -> (bool, Option<String>) {
        match action {
            ActionShell::SetVolume { percent } => (
                proc::ok(
                    "wpctl",
                    &[
                        "set-volume",
                        "-l",
                        "1",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{percent}%"),
                    ],
                ),
                None,
            ),
            ActionShell::SetBrightness { percent } => (
                proc::ok("brightnessctl", &["set", &format!("{percent}%")]),
                None,
            ),

            ActionShell::WifiSetEnabled { on } => (
                proc::ok("nmcli", &["radio", "wifi", if on { "on" } else { "off" }]),
                None,
            ),
            ActionShell::WifiScan => (proc::ok("nmcli", &["device", "wifi", "rescan"]), None),
            ActionShell::WifiConnect { ssid } => self.wifi_connect(&ssid),

            ActionShell::BtSetPowered { on } => (
                proc::ok("bluetoothctl", &["power", if on { "on" } else { "off" }]),
                None,
            ),
            // Timed discovery, so devices that only advertise while scanning
            // still show up. Fire and forget: nothing waits on the result.
            ActionShell::BtScan => {
                proc::detach("bluetoothctl --timeout 10 scan on >/dev/null 2>&1");
                (true, None)
            }
            ActionShell::BtConnect { mac, .. } => {
                // Trust first so it reconnects by itself next time. No explicit
                // `pair`: BlueZ completes any missing pairing during connect,
                // and `pair` fails outright on a device not in pairing mode.
                proc::ok("bluetoothctl", &["trust", &mac]);
                (proc::ok("bluetoothctl", &["connect", &mac]), None)
            }
            ActionShell::BtDisconnect { mac, .. } => {
                (proc::ok("bluetoothctl", &["disconnect", &mac]), None)
            }

            ActionShell::Launch { cmd } => {
                proc::detach(&format!("cd ~ && {cmd}"));
                (true, None)
            }

            ActionShell::Capture => self.capture(),
            ActionShell::CopyShot { path } => (
                proc::pipe_file("wl-copy", &["--type", "image/png"], &path).unwrap_or(false),
                None,
            ),

            ActionShell::Notify { title, body } => {
                (proc::ok("notify-send", &[&title, &body]), None)
            }
        }
    }

    /// Join a network: bring up a saved connection, else try an open join, else
    /// ask for the passphrase.
    fn wifi_connect(&self, ssid: &str) -> (bool, Option<String>) {
        let saved = proc::out("nmcli", &["-t", "-f", "NAME", "connection", "show"])
            .map(|o| o.lines().any(|l| l == ssid))
            .unwrap_or(false);
        if saved {
            return (proc::ok("nmcli", &["connection", "up", "id", ssid]), None);
        }
        if proc::ok("nmcli", &["device", "wifi", "connect", ssid]) {
            return (true, None);
        }
        let theme = format!(
            "{}/rofi/themes/menu.rasi",
            self.paths()
                .eww_config
                .parent()
                .map(Paths::str)
                .unwrap_or_default()
        );
        let pass = proc::out(
            "rofi",
            &["-dmenu", "-password", "-theme", &theme, "-p", ssid],
        );
        match pass {
            // A dismissed prompt is a deliberate no-op, not a failure. The
            // machine has a rule keyed on this exact word.
            None => (false, Some("cancelled".into())),
            Some(p) if p.is_empty() => (false, Some("cancelled".into())),
            Some(p) => (
                proc::ok(
                    "nmcli",
                    &["device", "wifi", "connect", ssid, "password", &p],
                ),
                None,
            ),
        }
    }

    /// Region screenshot.
    ///
    /// The whole screen is grabbed *before* the selector draws, then cropped to
    /// the chosen region. `grim -g "$(slurp)"` races: grim can grab a frame in
    /// which the compositor has not yet torn down slurp's overlay, baking the
    /// blue selection into the shot. Grabbing first makes that impossible.
    fn capture(&self) -> (bool, Option<String>) {
        let dir = &self.paths().shots;
        let _ = std::fs::create_dir_all(dir);
        reap_shots(dir);

        let stamp = proc::out("date", &["+%s%N"]).unwrap_or_default();
        let full = dir.join(format!("full-{stamp}.png"));
        let full_s = Paths::str(&full);
        if !proc::ok("grim", &[&full_s]) {
            return (false, None);
        }

        let accent = crate::accent_hex();
        let geom = proc::out(
            "slurp",
            &[
                "-b",
                "00000040",
                "-c",
                &format!("{accent}ff"),
                "-s",
                &format!("{accent}26"),
                "-w",
                "2",
            ],
        );
        let Some(geom) = geom.filter(|g| !g.is_empty()) else {
            let _ = std::fs::remove_file(&full);
            return (false, Some("cancelled".into()));
        };
        let Some((x, y, width, height)) = parse_geometry(&geom) else {
            let _ = std::fs::remove_file(&full);
            return (false, None);
        };

        let shot = dir.join(format!("shot-{stamp}.png"));
        let shot_s = Paths::str(&shot);
        let cropped = proc::ok(
            "magick",
            &[
                &full_s,
                "-crop",
                &format!("{width}x{height}+{x}+{y}"),
                "+repage",
                &shot_s,
            ],
        );
        let _ = std::fs::remove_file(&full);
        if !cropped {
            return (false, None);
        }

        self.send(Event::FromJob(EventJob::ShotStaged {
            shot: Shot {
                path: shot_s,
                width,
                height,
            },
        }));
        (true, None)
    }
}

/// Drop shots left over from previous runs; none of this run's files exist yet.
fn reap_shots(dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.path().extension().is_some_and(|x| x == "png") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// slurp prints `X,Y WxH`.
fn parse_geometry(geom: &str) -> Option<(u32, u32, u32, u32)> {
    let (xy, wh) = geom.trim().split_once(' ')?;
    let (x, y) = xy.split_once(',')?;
    let (w, h) = wh.split_once('x')?;
    Some((
        x.parse().ok()?,
        y.parse().ok()?,
        w.parse().ok()?,
        h.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_geometry;

    #[test]
    fn slurp_geometry() {
        assert_eq!(
            parse_geometry("100,200 640x480"),
            Some((100, 200, 640, 480))
        );
        assert_eq!(parse_geometry(""), None);
        assert_eq!(parse_geometry("garbage"), None);
    }
}
