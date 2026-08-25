//! The control socket: how a widget click reaches the state machine.
//!
//! One line in, one reply out. A widget handler is `eww-state menu toggle wifi`
//! — connect, write, read, exit — which is fast enough that eww's 200ms command
//! timeout is never in play. That timeout is what used to truncate the shell
//! handlers mid-way and leave the state lying about what was on screen.

use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::time::Duration;

use module_bar::Event;
use module_bar::EventBar;
use module_bar::MenuId;
use module_bar::ModuleId;
use module_shot::EventShot;
use module_shot::ShotAction;

use crate::paths::Paths;
use crate::runtime_daemon::Msg;

#[derive(Debug)]
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
            .map(|menu| ev(Event::Bar(EventBar::MenuToggled { menu })))
            .ok_or_else(|| format!("unknown menu: {name}")),
        ["menu", "close"] => Ok(ev(Event::Bar(EventBar::MenuDismissed))),

        ["volume", "set", v] => {
            num(v).map(|percent| ev(Event::Audio(module_audio::EventAudio::Set { percent })))
        }
        ["brightness", "set", v] => num(v).map(|percent| {
            ev(Event::Backlight(module_backlight::EventBacklight::Set {
                percent,
            }))
        }),

        ["wifi", "toggle"] => Ok(ev(Event::Wifi(module_wifi::EventWifi::Toggled))),
        ["wifi", "rescan"] => Ok(ev(ModuleId::Wifi.due())),
        ["wifi", "connect", ssid_b64] => Ok(ev(Event::Wifi(module_wifi::EventWifi::Picked {
            ssid: decode(ssid_b64)?,
        }))),

        ["bt", "toggle"] => Ok(ev(Event::Bluetooth(
            module_bluetooth::EventBluetooth::Toggled,
        ))),
        ["bt", "connect", mac_b64] => Ok(ev(Event::Bluetooth(
            module_bluetooth::EventBluetooth::Picked {
                mac: decode(mac_b64)?,
            },
        ))),

        ["dock", "launch", id] => Ok(ev(Event::Dock(module_dock::EventDock::Launched {
            id: id.to_string(),
        }))),
        ["dock", "swap", moved, onto] => Ok(ev(Event::Dock(module_dock::EventDock::Dropped {
            moved: moved.to_string(),
            onto: onto.to_string(),
        }))),

        ["shot", "capture"] => Ok(ev(Event::Shot(EventShot::Requested))),
        ["shot", action] => ShotAction::parse(action)
            .map(|action| ev(Event::Shot(EventShot::Acted { action })))
            .ok_or_else(|| format!("unknown shot action: {action}")),

        ["refresh", module] => ModuleId::parse(module)
            .map(|id| ev(id.due()))
            .ok_or_else(|| format!("unknown module: {module}")),

        _ => usage(),
    }
}

fn ev(event: Event) -> Cmd {
    Cmd::Ev(event)
}

fn num(v: &str) -> Result<u8, String> {
    // Sliders hand us floats ("42.0"); take the integer part.
    let v = v.split_once('.').map(|(a, _)| a).unwrap_or(v);
    v.parse::<u8>()
        .or_else(|_| v.parse::<f64>().map(|f| f.clamp(0.0, 100.0) as u8))
        .map_err(|_| format!("not a number: {v}"))
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

/// Refuse to start a second daemon, but clean up after one that died.
pub fn bind(paths: &Paths) -> std::io::Result<UnixListener> {
    if paths.socket.exists() {
        if request(&paths.socket, &["ping".to_string()]).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AddrInUse,
                "eww-state is already running",
            ));
        }
        let _ = std::fs::remove_file(&paths.socket);
    }
    UnixListener::bind(&paths.socket)
}

/// Answer connections until the listener dies.
pub fn serve(listener: UnixListener, tx: Sender<Msg>) {
    let spawned = std::thread::Builder::new()
        .name("ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let tx = tx.clone();
                // A connection per click, each handled on its own thread: a
                // client that stops reading can't stall anyone else.
                let spawned = std::thread::Builder::new()
                    .name("ipc-conn".into())
                    .spawn(move || handle(stream, tx));
                if let Err(e) = spawned {
                    tracing::error!(error = %e, "could not answer a connection");
                }
            }
        });
    if let Err(e) = spawned {
        tracing::error!(error = %e, "could not listen on the control socket");
    }
}

fn handle(stream: UnixStream, tx: Sender<Msg>) {
    let mut line = String::new();
    let Ok(clone) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(clone);
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let args: Vec<&str> = line.trim_end_matches('\n').split('\t').collect();
    let reply = match parse(&args) {
        Err(e) => format!("error: {e}\n"),
        Ok(Cmd::Ping) => "ok\n".to_string(),
        Ok(Cmd::Ev(event)) => match tx.send(Msg::Ev(event)) {
            Ok(()) => "ok\n".to_string(),
            Err(_) => "error: daemon shutting down\n".to_string(),
        },
        Ok(Cmd::Query(kind)) => {
            let (reply_tx, reply_rx) = channel();
            let sent = tx.send(Msg::Query {
                kind,
                reply: reply_tx,
            });
            if sent.is_err() {
                "error: daemon shutting down\n".to_string()
            } else {
                reply_rx
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap_or_else(|_| "error: no answer\n".to_string())
            }
        }
    };
    let mut out = stream;
    let _ = out.write_all(reply.as_bytes());
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

    /// Assert that a command line parses to something matching `pat`.
    ///
    /// A pattern rather than an equality check, because an event deliberately
    /// carries no `PartialEq` — see docs/STATE_MACHINES.md.
    macro_rules! parses {
        ($args:expr, $pat:pat $(if $guard:expr)?) => {
            let parsed = parse(&$args).expect("should parse");
            assert!(
                matches!(parsed, $pat $(if $guard)?),
                "{:?} did not match `{}`",
                parsed,
                stringify!($pat)
            );
        };
    }

    #[test]
    fn menu_commands() {
        parses!(
            ["menu", "toggle", "wifi"],
            Cmd::Ev(Event::Bar(EventBar::MenuToggled { menu: MenuId::Wifi }))
        );
        parses!(
            ["menu", "close"],
            Cmd::Ev(Event::Bar(EventBar::MenuDismissed))
        );
        assert!(parse(&["menu", "toggle", "nope"]).is_err());
    }

    #[test]
    fn slider_values_may_be_floats() {
        parses!(
            ["volume", "set", "42"],
            Cmd::Ev(Event::Audio(module_audio::EventAudio::Set { percent: 42 }))
        );
        parses!(
            ["volume", "set", "42.0"],
            Cmd::Ev(Event::Audio(module_audio::EventAudio::Set { percent: 42 }))
        );
        assert!(parse(&["volume", "set", "loud"]).is_err());
    }

    #[test]
    fn names_arrive_encoded() {
        // "my:net" — a raw SSID could carry anything, including a quote.
        parses!(
            ["wifi", "connect", "bXk6bmV0"],
            Cmd::Ev(Event::Wifi(module_wifi::EventWifi::Picked { ref ssid }))
                if ssid == "my:net"
        );
        assert!(parse(&["wifi", "connect", "not base64!"]).is_err());
    }

    #[test]
    fn base64_roundtrips_with_the_encoder_widgets_use() {
        for s in ["", "a", "ab", "abc", "Café ✳", "AA:BB:CC:DD:EE:FF"] {
            let encoded = api::util::b64(s);
            assert_eq!(decode(&encoded).expect("valid base64"), s, "roundtrip {s}");
        }
    }

    #[test]
    fn shot_commands() {
        parses!(
            ["shot", "capture"],
            Cmd::Ev(Event::Shot(EventShot::Requested))
        );
        parses!(
            ["shot", "copy"],
            Cmd::Ev(Event::Shot(EventShot::Acted {
                action: ShotAction::Copy
            }))
        );
        parses!(
            ["shot", "close"],
            Cmd::Ev(Event::Shot(EventShot::Acted {
                action: ShotAction::Dismiss
            }))
        );
        assert!(parse(&["shot", "eat"]).is_err());
    }

    /// A rescan is not its own command: it is the same "you are due" the timer
    /// sends, which is why the module cannot tell the two apart.
    #[test]
    fn a_rescan_is_a_module_falling_due() {
        parses!(
            ["wifi", "rescan"],
            Cmd::Ev(Event::Wifi(module_wifi::EventWifi::Due))
        );
        parses!(
            ["refresh", "bt"],
            Cmd::Ev(Event::Bluetooth(module_bluetooth::EventBluetooth::Due))
        );
        assert!(parse(&["refresh", "nope"]).is_err());
    }

    #[test]
    fn queries_and_unknowns() {
        parses!(["dump"], Cmd::Query(Query::Dump));
        parses!(["ping"], Cmd::Ping);
        assert!(parse(&["explode"]).is_err());
        assert!(parse(&[]).is_err());
    }
}
