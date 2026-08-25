//! Reading Wi-Fi state out of nmcli.
//!
//! Split in two on purpose: `sample` runs the program, and the `parse_*`
//! functions below turn its output into our own types. All the fiddly knowledge
//! — nmcli's escaping, the way a network appears once per band — is in the
//! parsers, which are tested here without running anything.

use crate::event::EventWifi;
use crate::model::NetSnapshot;
use crate::model::WifiNetwork;

pub fn sample() -> EventWifi {
    let enabled = exec::out("nmcli", &["-t", "-f", "WIFI", "radio"]).as_deref() == Some("enabled");
    let wired = exec::out("nmcli", &["-t", "-f", "STATE,TYPE", "device"])
        .map(|o| parse_wired(&o))
        .unwrap_or(false);
    if !enabled {
        return EventWifi::Observed {
            net: NetSnapshot {
                enabled,
                wired,
                ..Default::default()
            },
        };
    }
    let list = exec::out(
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
    let mut net = parse_list(&list);
    net.enabled = true;
    net.wired = wired;
    EventWifi::Observed { net }
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
fn parse_list(out: &str) -> NetSnapshot {
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
                push(&mut fields, c);
                escaped = false;
            }
            c if c == sep => fields.push(String::new()),
            c => push(&mut fields, c),
        }
    }
    fields
}

fn push(fields: &mut [String], c: char) {
    let Some(last) = fields.last_mut() else {
        unreachable!("the vector is created with one field and never shrinks")
    };
    last.push(c);
}

#[cfg(test)]
mod tests {
    use super::parse_list;
    use super::parse_wired;

    #[test]
    fn ssids_with_colons_survive() {
        let out = "*:72:WPA2:my\\:net\n:45:WPA2:other\n:90::open";
        let net = parse_list(out);
        assert_eq!(net.connected.as_deref(), Some("my:net"));
        assert_eq!(net.networks.len(), 3);
        assert!(net.networks[0].active);
        assert!(!net.networks[2].secure, "no security field means open");
    }

    #[test]
    fn bands_dedupe_and_the_active_one_is_found_anywhere() {
        // The connected BSS is the weaker of two rows for the same SSID.
        let out = ":88:WPA2:home\n*:41:WPA2:home\n:30:WPA2:neighbour";
        let net = parse_list(out);
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
}
