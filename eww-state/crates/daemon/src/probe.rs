//! Sampling the world.
//!
//! Every probe is split in two: a pure `parse_*` function that turns a
//! program's output into a core observation, and a thin `sample_*` that runs
//! the program. The parsers carry all the fiddly knowledge (nmcli's escaping,
//! wpctl's mute marker) and are unit-tested here without running anything.

use eww_state_core::{
    Battery, BtDevice, BtSnapshot, DockApp, NetSnapshot, Obs, Probe, Theme, WifiNetwork,
};

use crate::paths::Paths;
use crate::proc;

pub fn sample(probe: Probe, paths: &Paths) -> Option<Obs> {
    match probe {
        Probe::Clock => Some(Obs::Clock(
            proc::out("date", &["+%a %b %e  %H:%M"]).unwrap_or_default(),
        )),
        Probe::Battery => Some(Obs::Battery(sample_battery())),
        Probe::Theme => Some(Obs::Theme(parse_theme(
            &proc::out(
                "gsettings",
                &["get", "org.gnome.desktop.interface", "color-scheme"],
            )
            .unwrap_or_default(),
        ))),
        Probe::Audio => Some(Obs::Audio(parse_volume(&proc::out(
            "wpctl",
            &["get-volume", "@DEFAULT_AUDIO_SINK@"],
        )?))),
        Probe::Backlight => Some(Obs::Backlight(parse_brightness(&proc::out(
            "brightnessctl",
            &["-m"],
        )?)?)),
        Probe::Net => Some(Obs::Net(sample_net())),
        Probe::Bt => Some(Obs::Bt(sample_bt())),
        Probe::Dock => Some(Obs::Dock(read_dock(paths))),
        Probe::Fullscreen => Some(Obs::Fullscreen(parse_fullscreen(&proc::out(
            "hyprctl",
            &["activeworkspace"],
        )?))),
    }
}

// ---------------------------------------------------------------- battery

fn sample_battery() -> Battery {
    let read = |f: &str| std::fs::read_to_string(format!("/sys/class/power_supply/BAT0/{f}")).ok();
    let percent = read("capacity")
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(0)
        .min(100);
    let status = read("status").unwrap_or_default();
    Battery {
        percent,
        charging: matches!(status.trim(), "Charging" | "Full"),
    }
}

// ------------------------------------------------------------------ theme

fn parse_theme(out: &str) -> Theme {
    if out.to_ascii_lowercase().contains("dark") {
        Theme::Dark
    } else {
        Theme::Light
    }
}

// ------------------------------------------------------------------ audio

/// `wpctl get-volume` prints `Volume: 0.35` and appends `[MUTED]` when muted.
fn parse_volume(out: &str) -> eww_state_core::Audio {
    let level = out
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    eww_state_core::Audio {
        volume: (level * 100.0).round().clamp(0.0, 100.0) as u8,
        muted: out.contains("MUTED"),
    }
}

/// `brightnessctl -m` prints `device,class,current,percent,max`.
fn parse_brightness(out: &str) -> Option<u8> {
    let line = out.lines().next()?;
    let pct = line.split(',').nth(3)?.trim_end_matches('%');
    pct.parse::<u8>().ok()
}

// -------------------------------------------------------------- networking

fn sample_net() -> NetSnapshot {
    let enabled = proc::out("nmcli", &["-t", "-f", "WIFI", "radio"]).as_deref() == Some("enabled");
    let wired = proc::out("nmcli", &["-t", "-f", "STATE,TYPE", "device"])
        .map(|o| parse_wired(&o))
        .unwrap_or(false);
    if !enabled {
        return NetSnapshot {
            enabled,
            wired,
            ..Default::default()
        };
    }
    let list = proc::out(
        "nmcli",
        &[
            "-t",
            "-f",
            "IN-USE,SIGNAL,SECURITY,SSID",
            "device",
            "wifi",
            "list",
        ],
    )
    .unwrap_or_default();
    let mut net = parse_wifi_list(&list);
    net.enabled = true;
    net.wired = wired;
    net
}

fn parse_wired(out: &str) -> bool {
    out.lines()
        .any(|l| l.starts_with("connected:") && l.ends_with("ethernet"))
}

/// One row per SSID, strongest first, with the in-use network marked.
///
/// A network can appear once per band, and the BSS carrying the `*` is not
/// necessarily the strongest one — so the connected SSID is found across all
/// rows before the list is deduplicated.
fn parse_wifi_list(out: &str) -> NetSnapshot {
    let rows: Vec<Vec<String>> = out
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| split_escaped(l, ':'))
        .filter(|f| f.len() >= 4)
        .collect();

    let connected = rows
        .iter()
        .find(|f| f[0] == "*")
        .map(|f| f[3].clone())
        .filter(|s| !s.is_empty());

    let mut networks: Vec<WifiNetwork> = Vec::new();
    for f in &rows {
        let ssid = f[3].clone();
        if ssid.is_empty() || networks.iter().any(|n| n.ssid == ssid) {
            continue;
        }
        let signal = f[1].parse::<u8>().unwrap_or(0);
        networks.push(WifiNetwork {
            active: Some(&ssid) == connected.as_ref(),
            ssid,
            signal,
            secure: !f[2].is_empty(),
        });
    }

    NetSnapshot {
        enabled: true,
        connected,
        networks,
        wired: false,
    }
}

/// Split an `nmcli -t` line on unescaped separators, unescaping as it goes.
fn split_escaped(line: &str, sep: char) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for c in line.chars() {
        match c {
            '\\' if !escaped => escaped = true,
            c if escaped => {
                fields.last_mut().expect("never empty").push(c);
                escaped = false;
            }
            c if c == sep => fields.push(String::new()),
            c => fields.last_mut().expect("never empty").push(c),
        }
    }
    fields
}

// --------------------------------------------------------------- bluetooth

fn sample_bt() -> BtSnapshot {
    let powered = proc::out("bluetoothctl", &["show"])
        .map(|o| parse_powered(&o))
        .unwrap_or(false);
    if !powered {
        return BtSnapshot::default();
    }
    // No single list is reliable: bluetoothctl reports some known devices as
    // Trusted with Paired/Bonded = no. The union, deduped, is "My Devices".
    let mut listing = String::new();
    for kind in ["Paired", "Bonded", "Trusted", "Connected"] {
        if let Some(o) = proc::out("bluetoothctl", &["devices", kind]) {
            listing.push_str(&o);
            listing.push('\n');
        }
    }
    let connected = proc::out("bluetoothctl", &["devices", "Connected"]).unwrap_or_default();
    BtSnapshot {
        powered,
        devices: parse_devices(&listing, &connected),
    }
}

fn parse_powered(show: &str) -> bool {
    show.lines()
        .find_map(|l| l.trim().strip_prefix("Powered:"))
        .map(|v| v.trim() == "yes")
        .unwrap_or(false)
}

/// `bluetoothctl devices` prints `Device AA:BB:CC:DD:EE:FF Name With Spaces`.
fn parse_devices(listing: &str, connected: &str) -> Vec<BtDevice> {
    let connected_macs: Vec<&str> = connected
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    let mut devices: Vec<BtDevice> = Vec::new();
    for line in listing.lines() {
        let mut parts = line.splitn(3, ' ');
        if parts.next() != Some("Device") {
            continue;
        }
        let Some(mac) = parts.next() else { continue };
        if devices.iter().any(|d| d.mac == mac) {
            continue;
        }
        devices.push(BtDevice {
            mac: mac.to_string(),
            name: parts.next().unwrap_or(mac).to_string(),
            connected: connected_macs.contains(&mac),
        });
    }
    devices
}

// -------------------------------------------------------------------- dock

/// `dock.conf` is `id<TAB>command<TAB>icon`, one app per line, in dock order.
/// The file *is* the order — there is no second list to disagree with it.
pub fn read_dock(paths: &Paths) -> Vec<DockApp> {
    let text = std::fs::read_to_string(&paths.dock_conf).unwrap_or_default();
    parse_dock(&text)
}

fn parse_dock(text: &str) -> Vec<DockApp> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t').map(str::trim).filter(|s| !s.is_empty());
            Some(DockApp {
                id: f.next()?.to_string(),
                cmd: f.next()?.to_string(),
                icon: f.next()?.to_string(),
            })
        })
        .collect()
}

pub fn write_dock(paths: &Paths, apps: &[DockApp]) -> std::io::Result<()> {
    let mut text = String::from("# id\tcommand\ticon — order here is dock order\n");
    for a in apps {
        text.push_str(&format!("{}\t{}\t{}\n", a.id, a.cmd, a.icon));
    }
    // Write-then-rename: a crash mid-write must not leave a truncated dock.
    let tmp = paths.dock_conf.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &paths.dock_conf)
}

// -------------------------------------------------------------- compositor

fn parse_fullscreen(out: &str) -> bool {
    out.lines()
        .find_map(|l| l.trim().strip_prefix("hasfullscreen:"))
        .map(|v| v.trim() != "0")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_and_mute() {
        let a = parse_volume("Volume: 0.35");
        assert_eq!((a.volume, a.muted), (35, false));
        let a = parse_volume("Volume: 0.60 [MUTED]");
        assert_eq!((a.volume, a.muted), (60, true));
        assert_eq!(parse_volume("").volume, 0);
    }

    #[test]
    fn brightness_percent() {
        assert_eq!(
            parse_brightness("amdgpu_bl1,backlight,133,52%,255"),
            Some(52)
        );
        assert_eq!(parse_brightness(""), None);
    }

    #[test]
    fn wifi_ssids_with_colons_survive() {
        let out = "*:72:WPA2:my\\:net\n:45:WPA2:other\n:90::open";
        let net = parse_wifi_list(out);
        assert_eq!(net.connected.as_deref(), Some("my:net"));
        assert_eq!(net.networks.len(), 3);
        assert!(net.networks[0].active);
        assert!(!net.networks[2].secure, "no security field means open");
    }

    #[test]
    fn wifi_dedupes_bands_and_finds_the_active_one_anywhere() {
        // The connected BSS is the weaker of two rows for the same SSID.
        let out = ":88:WPA2:home\n*:41:WPA2:home\n:30:WPA2:neighbour";
        let net = parse_wifi_list(out);
        assert_eq!(net.networks.len(), 2);
        assert_eq!(net.networks[0].ssid, "home");
        assert_eq!(net.networks[0].signal, 88, "strongest row wins the list");
        assert!(net.networks[0].active, "and is still marked connected");
    }

    #[test]
    fn wired_only_counts_when_connected() {
        assert!(parse_wired("connected:ethernet"));
        assert!(!parse_wired("disconnected:ethernet\nconnected:wifi"));
    }

    #[test]
    fn bluetooth_union_is_deduped_and_names_keep_spaces() {
        let listing =
            "Device AA:BB Sony WH-1000XM4\nDevice AA:BB Sony WH-1000XM4\nDevice CC:DD Mouse";
        let devices = parse_devices(listing, "Device CC:DD Mouse");
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "Sony WH-1000XM4");
        assert!(!devices[0].connected);
        assert!(devices[1].connected);
    }

    #[test]
    fn powered_flag() {
        assert!(parse_powered(
            "Controller X\n\tPowered: yes\n\tDiscoverable: no"
        ));
        assert!(!parse_powered("Controller X\n\tPowered: no"));
        assert!(!parse_powered(""));
    }

    #[test]
    fn theme_detection() {
        assert_eq!(parse_theme("'prefer-dark'"), Theme::Dark);
        assert_eq!(parse_theme("'default'"), Theme::Light);
    }

    #[test]
    fn fullscreen_flag() {
        assert!(parse_fullscreen(
            "workspace ID 2 (2) on monitor eDP-1:\n\tmonitorID: 0\n\thasfullscreen: 1"
        ));
        assert!(!parse_fullscreen("\thasfullscreen: 0"));
        assert!(!parse_fullscreen(""));
    }

    #[test]
    fn dock_conf_roundtrip() {
        let text =
            "# comment\n\nfirefox\tfirefox\t/i/ff.png\nterm\talacritty\t/i/term.svg\nbroken\n";
        let apps = parse_dock(text);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "firefox");
        assert_eq!(apps[1].cmd, "alacritty");
    }
}
