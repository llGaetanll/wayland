//! Carrying out what the Wi-Fi module asked for.
//!
//! Nothing here decides anything. Where a command needs a second try or a
//! prompt, that is how the command works, not a rule about the widget: the
//! outcome goes back as an event and the module owns what it means.
//!
//! Each action answers with the event it produces, or with nothing when it is
//! one the module never waits on. That is why the runtime carrying this out
//! needs to know nothing about Wi-Fi.

use crate::action::ActionWifi;
use crate::event::EventWifi;
use crate::probe;

/// Where to ask the user for a passphrase. The module knows nothing about rofi;
/// the runtime hands this in.
pub struct Prompt {
    pub theme: String,
}

pub fn perform(action: &ActionWifi, prompt: &Prompt) -> Option<EventWifi> {
    match action {
        ActionWifi::Sample => Some(probe::sample()),
        ActionWifi::SetEnabled { id, on } => {
            let ok = exec::ok("nmcli", &["radio", "wifi", if *on { "on" } else { "off" }]);
            Some(EventWifi::Finished {
                id: *id,
                ok,
                detail: None,
            })
        }
        ActionWifi::Scan => {
            exec::ok("nmcli", &["device", "wifi", "rescan"]);
            None
        }
        ActionWifi::Connect { id, ssid } => {
            let (ok, detail) = connect(ssid, prompt);
            Some(EventWifi::Finished {
                id: *id,
                ok,
                detail,
            })
        }
        ActionWifi::Notify { body } => {
            exec::ok("notify-send", &["Wi-Fi", body]);
            None
        }
    }
}

/// Join a network: bring up a saved connection, else try an open join, else ask
/// for the passphrase.
fn connect(ssid: &str, prompt: &Prompt) -> (bool, Option<String>) {
    let saved = exec::out("nmcli", &["-t", "-f", "NAME", "connection", "show"])
        .map(|o| o.lines().any(|l| l == ssid))
        .unwrap_or(false);
    if saved {
        return (exec::ok("nmcli", &["connection", "up", "id", ssid]), None);
    }
    if exec::ok("nmcli", &["device", "wifi", "connect", ssid]) {
        return (true, None);
    }
    let pass = exec::out(
        "rofi",
        &["-dmenu", "-password", "-theme", &prompt.theme, "-p", ssid],
    );
    match pass {
        // A dismissed prompt is a deliberate no-op, not a failure. The module
        // has a rule keyed on this exact word.
        None => (false, Some("cancelled".into())),
        Some(p) if p.is_empty() => (false, Some("cancelled".into())),
        Some(p) => (
            exec::ok(
                "nmcli",
                &["device", "wifi", "connect", ssid, "password", &p],
            ),
            None,
        ),
    }
}
