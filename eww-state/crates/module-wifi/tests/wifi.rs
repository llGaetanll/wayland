//! What the Wi-Fi module does, said as the conversations it can have.
//!
//! Nothing here reaches inside `StateWifi`: a test asks the same questions the
//! bar asks — the actions that came out, the vars that go to eww, whether it
//! wants dismissing.

use api::Millis;
use api::Module;
use api::WireVars;
use eval::eval;
use module_wifi::event::EventWifi;
use module_wifi::model::NetSnapshot;
use module_wifi::model::WifiNetwork;
use module_wifi::ActionWifi;
use module_wifi::ConfigWifi;
use module_wifi::StateWifi;

fn wifi() -> StateWifi {
    StateWifi::new(ConfigWifi::default())
}

fn seen(ssids: &[(&str, u8, bool)]) -> EventWifi {
    EventWifi::Observed {
        net: NetSnapshot {
            enabled: true,
            connected: ssids
                .iter()
                .find(|(_, _, active)| *active)
                .map(|(ssid, _, _)| ssid.to_string()),
            networks: ssids
                .iter()
                .map(|(ssid, signal, active)| WifiNetwork {
                    ssid: ssid.to_string(),
                    signal: *signal,
                    secure: true,
                    active: *active,
                })
                .collect(),
            wired: false,
        },
    }
}

/// The id a `Connect` went out with, so a test can answer that exact job.
fn joining_id(actions: &[ActionWifi]) -> api::JobId {
    actions
        .iter()
        .find_map(|a| match a {
            ActionWifi::Connect { id, .. } => Some(*id),
            _ => None,
        })
        .expect("a join was dispatched")
}

fn var(w: &StateWifi, name: &str) -> String {
    let mut vars = WireVars::default();
    w.vars(&mut vars);
    vars.get(name).unwrap_or_default().to_string()
}

/// Flipping the switch used to flicker back on: the radio changed, then a poll
/// that was already running reported the old state.
#[test]
fn a_sample_from_before_the_flip_does_not_undo_it() {
    let mut w = wifi();
    eval!(&mut w,
        < seen(&[("home", 90, true)]),

        < EventWifi::Toggled,
        > ActionWifi::SetEnabled { on: false, .. },

        // A sample taken before the radio actually went down.
        < seen(&[("home", 90, true)]),
    );
    assert!(var(&w, "wifi").contains("\"enabled\":false"));
    assert!(
        w.snapshot().networks.is_empty(),
        "ghosts are cleared at once"
    );
}

/// Two clicks landing together are one situation, not two: the switch ends up
/// where the second one left it, and the radio is asked once.
#[test]
fn two_clicks_in_one_batch_are_one_command() {
    let mut w = wifi();
    eval!(&mut w,
        < seen(&[("home", 90, true)]),

        < EventWifi::Toggled,
        < EventWifi::Toggled,
        > ActionWifi::SetEnabled { on: true, .. },
        > ActionWifi::Scan,
    );
}

#[test]
fn joining_says_so_then_asks_to_be_dismissed() {
    let mut w = wifi();
    let mut actions = Vec::new();
    w.handle_events(&[seen(&[("cafe", 60, false)])]);
    w.emit_actions(&mut actions);

    actions.clear();
    w.handle_events(&[EventWifi::Picked {
        ssid: "cafe".into(),
    }]);
    let req = w.emit_actions(&mut actions);
    assert!(req.is_none(), "the menu stays up while joining");
    assert_eq!(var(&w, "wifi_connecting"), "cafe");
    let id = joining_id(&actions);

    // A second click while the first is running is a mis-click.
    eval!(&mut w,
        < EventWifi::Picked { ssid: "other".into() },
    );

    eval!(&mut w,
        < EventWifi::Finished { id, ok: true, detail: None },
        > ActionWifi::Notify { .. },
        > ActionWifi::Sample,
    );
    assert_eq!(var(&w, "wifi_connecting"), "");
}

/// A dismissed passphrase prompt is a deliberate no-op, not a failure.
#[test]
fn a_cancelled_prompt_says_nothing() {
    let mut w = wifi();
    let mut actions = Vec::new();
    w.handle_events(&[EventWifi::Picked {
        ssid: "cafe".into(),
    }]);
    w.emit_actions(&mut actions);
    let id = joining_id(&actions);

    eval!(&mut w,
        < EventWifi::Finished { id, ok: false, detail: Some("cancelled".into()) },
        > ActionWifi::Sample,
    );
}

/// A command that never reports back used to leave the switch stuck mid-flip
/// for the rest of the session.
#[test]
fn a_job_that_never_answers_is_let_go_of() {
    let mut w = wifi();
    let timeout = ConfigWifi::default().job_timeout;
    eval!(&mut w,
        < EventWifi::Toggled,
        > ActionWifi::SetEnabled { on: true, .. },
        > ActionWifi::Scan,

        // Still inside the timeout, so the job is still believed in.
        < EventWifi::Tick { now: Millis(timeout.0 / 2) },
    );
    assert!(w.settling());

    eval!(&mut w,
        < EventWifi::Tick { now: timeout },
        > ActionWifi::Sample,
    );
    assert!(!w.settling());
    assert!(
        w.snapshot().enabled,
        "giving up on the job must not undo what the user asked for"
    );
}

#[test]
fn an_open_menu_polls_faster_than_the_bar_icon() {
    let mut w = wifi();
    let idle = w.poll_interval();
    eval!(&mut w,
        < EventWifi::MenuOpened,
        > ActionWifi::Scan,
        > ActionWifi::Sample,
    );
    assert!(w.poll_interval() < idle);
}

#[test]
fn ethernet_shows_when_wifi_has_no_link() {
    let mut w = wifi();
    let mut wired = NetSnapshot {
        enabled: true,
        ..Default::default()
    };
    wired.wired = true;
    eval!(&mut w,
        < EventWifi::Observed { net: wired },
    );
    assert!(var(&w, "net_icon").ends_with("network-wired-symbolic.svg"));
}
