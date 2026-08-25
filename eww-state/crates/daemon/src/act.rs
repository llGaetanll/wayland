//! Carrying out what the state machine asked for.
//!
//! Each action runs on its own thread and reports back as `Event::JobDone`, so
//! a ten-second `bluetoothctl connect` or a screenshot the user never finishes
//! selecting can't hold up anything else. Nothing here decides anything: the
//! outcome goes back to the machine, which owns what it means.

use std::sync::mpsc::Sender;

use eww_state_core::{Action, ActionKind, Event, Intent, JobId, Shot};

use crate::paths::Paths;
use crate::probe;
use crate::proc;
use crate::svg;

pub struct Runner {
    pub paths: Paths,
    pub events: Sender<Event>,
}

impl Runner {
    /// Run an action to completion. Called on a worker thread.
    pub fn run(&self, action: Action) {
        let Action { id, kind } = action;
        let (ok, detail) = self.execute(id, kind);
        let _ = self.events.send(Event::JobDone { id, ok, detail });
    }

    fn execute(&self, _id: JobId, kind: ActionKind) -> (bool, Option<String>) {
        match kind {
            ActionKind::Probe(p) => {
                if let Some(obs) = probe::sample(p, &self.paths) {
                    let _ = self.events.send(Event::Observed(obs));
                    (true, None)
                } else {
                    (false, None)
                }
            }

            ActionKind::SetVolume(v) => (
                proc::ok(
                    "wpctl",
                    &[
                        "set-volume",
                        "-l",
                        "1",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{v}%"),
                    ],
                ),
                None,
            ),
            ActionKind::SetBrightness(v) => {
                (proc::ok("brightnessctl", &["set", &format!("{v}%")]), None)
            }

            ActionKind::WifiSetEnabled(on) => (
                proc::ok("nmcli", &["radio", "wifi", if on { "on" } else { "off" }]),
                None,
            ),
            ActionKind::WifiScan => (proc::ok("nmcli", &["device", "wifi", "rescan"]), None),
            ActionKind::WifiConnect { ssid } => self.wifi_connect(&ssid),

            ActionKind::BtSetPowered(on) => (
                proc::ok("bluetoothctl", &["power", if on { "on" } else { "off" }]),
                None,
            ),
            // Timed discovery, so devices that only advertise while scanning
            // still show up. Fire and forget: nothing waits on the result.
            ActionKind::BtScan => {
                proc::detach("bluetoothctl --timeout 10 scan on >/dev/null 2>&1");
                (true, None)
            }
            ActionKind::BtConnect { mac, .. } => {
                // Trust first so it reconnects by itself next time. No explicit
                // `pair`: BlueZ completes any missing pairing during connect,
                // and `pair` fails outright on a device not in pairing mode.
                proc::ok("bluetoothctl", &["trust", &mac]);
                (proc::ok("bluetoothctl", &["connect", &mac]), None)
            }
            ActionKind::BtDisconnect { mac, .. } => {
                (proc::ok("bluetoothctl", &["disconnect", &mac]), None)
            }

            ActionKind::Launch { cmd } => {
                proc::detach(&format!("cd ~ && {cmd}"));
                (true, None)
            }
            ActionKind::PersistDock(apps) => (probe::write_dock(&self.paths, &apps).is_ok(), None),

            ActionKind::RenderBattery { percent, charging } => (
                svg::render_battery(&self.paths.cache, percent, charging).is_ok(),
                None,
            ),

            ActionKind::ShotCapture => self.capture(),
            ActionKind::ShotCopy { path } => (
                proc::pipe_file("wl-copy", &["--type", "image/png"], &path).unwrap_or(false),
                None,
            ),
            ActionKind::ShotSave { path } => {
                let name = proc::out("date", &["+Screenshot %Y-%m-%d %H.%M.%S.png"])
                    .unwrap_or_else(|| "Screenshot.png".into());
                let dest = self.paths.screenshots.join(name);
                (std::fs::copy(&path, &dest).is_ok(), None)
            }
            ActionKind::ShotDelete { path } => (std::fs::remove_file(&path).is_ok(), None),

            ActionKind::Notify { title, body } => (proc::ok("notify-send", &[&title, &body]), None),
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
            self.paths
                .eww_config
                .parent()
                .map(crate::paths::Paths::str)
                .unwrap_or_default()
        );
        let pass = proc::out(
            "rofi",
            &["-dmenu", "-password", "-theme", &theme, "-p", ssid],
        );
        match pass {
            // A dismissed prompt is a deliberate no-op, not a failure.
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
        let dir = &self.paths.shots;
        let _ = std::fs::create_dir_all(dir);
        reap_shots(dir);

        let stamp = proc::out("date", &["+%s%N"]).unwrap_or_default();
        let full = dir.join(format!("full-{stamp}.png"));
        let full_s = crate::paths::Paths::str(&full);
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
        let Some((x, y, w, h)) = parse_geometry(&geom) else {
            let _ = std::fs::remove_file(&full);
            return (false, None);
        };

        let shot = dir.join(format!("shot-{stamp}.png"));
        let shot_s = crate::paths::Paths::str(&shot);
        let cropped = proc::ok(
            "magick",
            &[
                &full_s,
                "-crop",
                &format!("{w}x{h}+{x}+{y}"),
                "+repage",
                &shot_s,
            ],
        );
        let _ = std::fs::remove_file(&full);
        if !cropped {
            return (false, None);
        }

        let _ = self.events.send(Event::Intent(Intent::ShotStage(Shot {
            path: shot_s,
            width: w,
            height: h,
        })));
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
