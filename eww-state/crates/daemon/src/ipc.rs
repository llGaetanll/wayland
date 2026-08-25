//! The control socket: how a widget click reaches the state machine.
//!
//! One line in, one reply out. A widget handler is `eww-state menu toggle wifi`
//! — connect, write, read, exit — which is fast enough that eww's 200ms command
//! timeout is never in play. That timeout is what used to truncate the shell
//! handlers mid-way and leave the state lying about what was on screen.

use std::io::{BufReader, Read, Write};
use std::os::unix::net::UnixStream;

use eww_state_core::{Event, Intent, MenuId, Probe, Shot, ShotAction};

#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    Ev(Event),
    Query(Query),
    Ping,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Query {
    Vars,
    Windows,
    Polls,
    Dump,
}

/// Parse a command line into something the daemon can act on.
///
/// Names that come from the network — SSIDs, device names — arrive base64
/// encoded, so nothing an access point can call itself is ever interpreted.
pub fn parse(args: &[&str]) -> Result<Cmd, String> {
    let usage = || Err(format!("bad command: {}", args.join(" ")));
    match args {
        ["ping"] => Ok(Cmd::Ping),
        ["vars"] => Ok(Cmd::Query(Query::Vars)),
        ["windows"] => Ok(Cmd::Query(Query::Windows)),
        ["polls"] => Ok(Cmd::Query(Query::Polls)),
        ["dump"] => Ok(Cmd::Query(Query::Dump)),

        ["menu", "toggle", name] => MenuId::parse(name)
            .map(|m| intent(Intent::MenuToggle(m)))
            .ok_or_else(|| format!("unknown menu: {name}")),
        ["menu", "close"] => Ok(intent(Intent::MenuClose)),

        ["volume", "set", v] => num(v).map(|n| intent(Intent::VolumeSet(n))),
        ["brightness", "set", v] => num(v).map(|n| intent(Intent::BrightnessSet(n))),

        ["wifi", "toggle"] => Ok(intent(Intent::WifiToggle)),
        ["wifi", "rescan"] => Ok(intent(Intent::Refresh(Probe::Net))),
        ["wifi", "connect", ssid_b64] => Ok(intent(Intent::WifiConnect {
            ssid: decode(ssid_b64)?,
        })),

        ["bt", "toggle"] => Ok(intent(Intent::BtToggle)),
        ["bt", "connect", mac_b64] => Ok(intent(Intent::BtConnect {
            mac: decode(mac_b64)?,
        })),

        ["dock", "launch", id] => Ok(intent(Intent::DockLaunch { id: id.to_string() })),
        ["dock", "swap", a, b] => Ok(intent(Intent::DockSwap {
            a: a.to_string(),
            b: b.to_string(),
        })),

        ["shot", "capture"] => Ok(intent(Intent::ShotCapture)),
        ["shot", "stage", path, w, h] => Ok(intent(Intent::ShotStage(Shot {
            path: path.to_string(),
            width: num32(w)?,
            height: num32(h)?,
        }))),
        ["shot", action] => ShotAction::parse(action)
            .map(|a| intent(Intent::ShotAct(a)))
            .ok_or_else(|| format!("unknown shot action: {action}")),

        ["refresh", probe] => Probe::parse(probe)
            .map(|p| intent(Intent::Refresh(p)))
            .ok_or_else(|| format!("unknown probe: {probe}")),

        _ => usage(),
    }
}

fn intent(i: Intent) -> Cmd {
    Cmd::Ev(Event::Intent(i))
}

fn num(v: &str) -> Result<u8, String> {
    // Sliders hand us floats ("42.0"); take the integer part.
    let v = v.split_once('.').map(|(a, _)| a).unwrap_or(v);
    v.parse::<u8>()
        .or_else(|_| v.parse::<f64>().map(|f| f.clamp(0.0, 100.0) as u8))
        .map_err(|_| format!("not a number: {v}"))
}

fn num32(v: &str) -> Result<u32, String> {
    v.parse().map_err(|_| format!("not a number: {v}"))
}

fn decode(input: &str) -> Result<String, String> {
    let bytes = b64_decode(input).ok_or_else(|| "invalid base64".to_string())?;
    String::from_utf8(bytes).map_err(|_| "invalid utf-8".to_string())
}

fn b64_decode(input: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::new();
    for c in input.trim().bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b'\n' | b'\r' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Send one command to a running daemon and return its reply.
pub fn request(socket: &std::path::Path, args: &[String]) -> std::io::Result<String> {
    let mut stream = UnixStream::connect(socket)?;
    stream.write_all(args.join("\t").as_bytes())?;
    stream.write_all(b"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reply = String::new();
    BufReader::new(stream).read_to_string(&mut reply)?;
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(args: &[&str]) -> Cmd {
        parse(args).expect("should parse")
    }

    #[test]
    fn menu_commands() {
        assert_eq!(
            ev(&["menu", "toggle", "wifi"]),
            intent(Intent::MenuToggle(MenuId::Wifi))
        );
        assert_eq!(ev(&["menu", "close"]), intent(Intent::MenuClose));
        assert!(parse(&["menu", "toggle", "nope"]).is_err());
    }

    #[test]
    fn slider_values_may_be_floats() {
        assert_eq!(ev(&["volume", "set", "42"]), intent(Intent::VolumeSet(42)));
        assert_eq!(
            ev(&["volume", "set", "42.0"]),
            intent(Intent::VolumeSet(42))
        );
        assert!(parse(&["volume", "set", "loud"]).is_err());
    }

    #[test]
    fn names_arrive_encoded() {
        // "my:net" — a raw SSID could carry anything, including a quote.
        assert_eq!(
            ev(&["wifi", "connect", "bXk6bmV0"]),
            intent(Intent::WifiConnect {
                ssid: "my:net".into()
            })
        );
        assert!(parse(&["wifi", "connect", "not base64!"]).is_err());
    }

    #[test]
    fn base64_roundtrips_with_the_encoder_widgets_use() {
        for s in ["", "a", "ab", "abc", "Café ✳", "AA:BB:CC:DD:EE:FF"] {
            let encoded = eww_state_core::util::b64(s);
            assert_eq!(decode(&encoded).unwrap(), s, "roundtrip {s}");
        }
    }

    #[test]
    fn shot_commands() {
        assert_eq!(ev(&["shot", "capture"]), intent(Intent::ShotCapture));
        assert_eq!(
            ev(&["shot", "copy"]),
            intent(Intent::ShotAct(ShotAction::Copy))
        );
        assert_eq!(
            ev(&["shot", "close"]),
            intent(Intent::ShotAct(ShotAction::Dismiss))
        );
        assert!(parse(&["shot", "eat"]).is_err());
    }

    #[test]
    fn queries_and_unknowns() {
        assert_eq!(ev(&["dump"]), Cmd::Query(Query::Dump));
        assert_eq!(ev(&["ping"]), Cmd::Ping);
        assert!(parse(&["explode"]).is_err());
        assert!(parse(&[]).is_err());
    }
}
