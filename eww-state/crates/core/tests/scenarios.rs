//! End-to-end scenarios, played against the fake world in `common`.
//!
//! Each of these is a bug the shell-script version actually had, written down
//! as the behaviour that replaces it.

mod common;

use common::World;
use eww_state_core::{
    Audio, BtDevice, BtSnapshot, Intent, MenuId, NetSnapshot, Obs, ShotAction, WifiNetwork,
};

fn networks(ssids: &[(&str, u8, bool)]) -> NetSnapshot {
    NetSnapshot {
        enabled: true,
        connected: ssids
            .iter()
            .find(|(_, _, a)| *a)
            .map(|(s, _, _)| s.to_string()),
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
    }
}

fn devices(list: &[(&str, &str, bool)]) -> BtSnapshot {
    BtSnapshot {
        powered: true,
        devices: list
            .iter()
            .map(|(mac, name, connected)| BtDevice {
                mac: mac.to_string(),
                name: name.to_string(),
                connected: *connected,
            })
            .collect(),
    }
}

/// The original sin: two independent `*_open` booleans meant two panels could
/// be on screen at once. Hammering every icon must never produce that.
#[test]
fn no_sequence_of_clicks_opens_two_menus() {
    let mut w = World::new();
    let clicks = [
        MenuId::Wifi,
        MenuId::Bt,
        MenuId::Bt,
        MenuId::Volume,
        MenuId::Wifi,
        MenuId::Wifi,
        MenuId::Brightness,
        MenuId::Volume,
        MenuId::Brightness,
        MenuId::Brightness,
    ];
    for (i, menu) in clicks.into_iter().enumerate() {
        w.intent(Intent::MenuToggle(menu));
        let open = w.menus_open();
        assert!(open.len() <= 1, "click {i} left {open:?} on screen");

        // The backdrop and the panel are always both there or both gone.
        let backdrop = w.windows().contains(&"menu-backdrop".to_string());
        assert_eq!(
            backdrop,
            open.len() == 1,
            "backdrop out of step at click {i}"
        );
    }
}

/// Clicking a second icon while one dropdown is open switches to it, rather
/// than stacking or getting stuck.
#[test]
fn switching_menus_swaps_the_panel_and_keeps_the_backdrop() {
    let mut w = World::new();
    w.intent(Intent::MenuToggle(MenuId::Wifi));
    assert_eq!(w.menus_open(), ["wifi-menu"]);
    w.intent(Intent::MenuToggle(MenuId::Bt));
    assert_eq!(w.menus_open(), ["bt-menu"]);
    assert!(w.windows().contains(&"menu-backdrop".to_string()));

    // Clicking outside dismisses whatever is up.
    w.intent(Intent::MenuClose);
    assert!(w.menus_open().is_empty());
    assert!(!w.windows().contains(&"menu-backdrop".to_string()));
}

/// A drag fires many times a second while a 2s poll is also running. The slider
/// used to snap back to a value from before the drag.
#[test]
fn dragging_the_volume_slider_beats_the_poll() {
    let mut w = World::new();
    w.intent(Intent::MenuToggle(MenuId::Volume));

    w.hold = true; // the setter has not returned yet
    for v in [55, 60, 65, 70] {
        w.intent(Intent::VolumeSet(v));
    }
    // A poll from before the drag lands in the middle of it.
    w.stale_sample(Obs::Audio(Audio {
        volume: 50,
        muted: false,
    }));
    assert_eq!(w.var("vol_pct"), "70", "the slider follows the finger");

    w.release();
    assert_eq!(w.var("vol_pct"), "70");
    assert_eq!(
        w.volume, 70,
        "and the world ends up where the finger left it"
    );
}

/// Joining a network: the row says so, then the menu dismisses itself and the
/// bar reflects the new connection.
#[test]
fn joining_a_network_end_to_end() {
    let mut w = World::new();
    w.net = networks(&[("home", 80, false), ("cafe", 40, false)]);
    w.intent(Intent::Refresh(eww_state_core::Probe::Net));
    w.intent(Intent::MenuToggle(MenuId::Wifi));

    w.hold = true;
    w.intent(Intent::WifiConnect {
        ssid: "home".into(),
    });
    assert_eq!(w.var("wifi_connecting"), "home");
    assert_eq!(
        w.menus_open(),
        ["wifi-menu"],
        "the menu stays up while joining"
    );

    w.release();
    assert_eq!(w.var("wifi_connecting"), "");
    assert!(
        w.menus_open().is_empty(),
        "a successful join dismisses itself"
    );
    assert_eq!(w.notifications(), ["Connected to home"]);
    assert!(w.var("wifi").contains("\"active\":true"));
    assert!(w.var("net_icon").ends_with("nm-signal-100-symbolic.svg"));
}

#[test]
fn a_failed_join_says_so_and_leaves_the_radio_alone() {
    let mut w = World::new();
    w.net = networks(&[("cafe", 40, false)]);
    w.wifi_fails = true;
    w.intent(Intent::Refresh(eww_state_core::Probe::Net));
    w.intent(Intent::MenuToggle(MenuId::Wifi));
    w.intent(Intent::WifiConnect {
        ssid: "cafe".into(),
    });

    assert_eq!(w.notifications(), ["Failed to connect to cafe"]);
    assert_eq!(w.var("wifi_connecting"), "");
    assert!(w.var("wifi").contains("\"enabled\":true"));
}

/// Turning the radio off used to flicker back on: the switch flipped, then a
/// poll that was already running reported the old state.
#[test]
fn the_radio_switch_does_not_flicker() {
    let mut w = World::new();
    w.net = networks(&[("home", 80, true)]);
    w.intent(Intent::Refresh(eww_state_core::Probe::Net));
    assert!(w.var("wifi").contains("\"enabled\":true"));

    w.hold = true;
    w.intent(Intent::WifiToggle);
    assert!(
        w.var("wifi").contains("\"enabled\":false"),
        "switch flips at once"
    );
    w.stale_sample(Obs::Net(networks(&[("home", 80, true)])));
    assert!(
        w.var("wifi").contains("\"enabled\":false"),
        "a sample from before the switch must not flip it back"
    );

    w.release();
    assert!(w.var("wifi").contains("\"enabled\":false"));
    assert!(!w.net.enabled);
}

/// Bluetooth is deliberately different from Wi-Fi: acting on a device leaves
/// the menu open so the row can update in place.
#[test]
fn connecting_a_device_keeps_the_menu_open() {
    let mut w = World::new();
    w.bt = devices(&[("AA:BB", "Buds", false)]);
    w.intent(Intent::Refresh(eww_state_core::Probe::Bt));
    w.intent(Intent::MenuToggle(MenuId::Bt));

    w.hold = true;
    w.intent(Intent::BtConnect {
        mac: "AA:BB".into(),
    });
    assert_eq!(w.var("bt_connecting"), "AA:BB");
    w.release();

    assert_eq!(w.menus_open(), ["bt-menu"]);
    assert_eq!(w.var("bt_connecting"), "");
    assert_eq!(w.notifications(), ["Connected Buds"]);
    assert!(w.var("bt_icon").ends_with("bluetooth-paired-symbolic.svg"));

    // Clicking the connected device again disconnects it.
    w.intent(Intent::BtConnect {
        mac: "AA:BB".into(),
    });
    assert_eq!(w.notifications().last(), Some(&"Disconnected Buds"));
    assert!(w.var("bt_icon").ends_with("bluetooth-active-symbolic.svg"));
}

/// A Print press must not disturb an open bar dropdown, in either direction.
#[test]
fn the_screenshot_preview_and_the_bar_menus_ignore_each_other() {
    let mut w = World::new();
    w.intent(Intent::MenuToggle(MenuId::Wifi));

    w.intent(Intent::ShotCapture);
    w.capture_finished("/shots/a.png", 1600, 900);
    assert_eq!(w.menus_open(), ["wifi-menu"], "the bar menu is untouched");
    assert!(w.windows().contains(&"shot-menu".to_string()));
    assert_eq!(w.var("shot_w"), "320", "preview is fitted, not stretched");
    assert_eq!(w.var("shot_h"), "180");

    // Clicking outside the preview dismisses only the preview.
    w.intent(Intent::ShotAct(ShotAction::Dismiss));
    assert!(!w.windows().contains(&"shot-menu".to_string()));
    assert_eq!(w.menus_open(), ["wifi-menu"]);

    // And dismissing the bar menu leaves no screenshot windows behind.
    w.intent(Intent::MenuClose);
    assert!(w.windows().iter().all(|n| n == "bar" || n == "dock"));
}

/// Mashing Print used to stack region selectors and overwrite the staged file.
#[test]
fn a_second_print_during_selection_is_ignored() {
    let mut w = World::new();
    w.intent(Intent::ShotCapture);
    w.intent(Intent::ShotCapture);
    w.intent(Intent::ShotCapture);
    assert_eq!(w.counts().get("shot-capture"), Some(&1));

    w.capture_finished("/shots/a.png", 800, 600);
    w.intent(Intent::ShotAct(ShotAction::Save));
    assert!(w.log.contains(&"shot-save /shots/a.png".to_string()));

    // With the first one finished, Print works again.
    w.intent(Intent::ShotCapture);
    assert_eq!(w.counts().get("shot-capture"), Some(&2));
}

/// The bar and the dock are separate windows, and used to be reconciled through
/// a single proxy — so once they disagreed, one of them got stranded.
#[test]
fn bar_and_dock_never_get_stranded_by_fullscreen() {
    let mut w = World::new();
    assert_eq!(w.windows(), ["bar", "dock"]);

    for _ in 0..3 {
        w.fullscreen = true;
        w.intent(Intent::Refresh(eww_state_core::Probe::Fullscreen));
        assert!(w.windows().is_empty(), "both hide together");

        w.fullscreen = false;
        w.intent(Intent::Refresh(eww_state_core::Probe::Fullscreen));
        assert_eq!(w.windows(), ["bar", "dock"], "and both come back");
    }
}

/// Opening a menu should cost a refresh; leaving it open should cost the faster
/// poll only while it is up.
#[test]
fn polling_follows_what_is_on_screen() {
    let mut w = World::new();
    let idle = w.bar.polls()[&eww_state_core::Probe::Net];

    w.intent(Intent::MenuToggle(MenuId::Wifi));
    let open = w.bar.polls()[&eww_state_core::Probe::Net];
    assert!(open < idle, "an open menu polls faster");
    assert!(
        w.log.contains(&"wifi-scan".to_string()),
        "and rescans on open"
    );

    w.intent(Intent::MenuClose);
    assert_eq!(w.bar.polls()[&eww_state_core::Probe::Net], idle);
}

/// eww restarting must not lose anything: the machine holds the truth and the
/// shell republishes it.
#[test]
fn state_survives_an_eww_restart() {
    let mut w = World::new();
    w.net = networks(&[("home", 80, true)]);
    w.intent(Intent::Refresh(eww_state_core::Probe::Net));
    w.intent(Intent::MenuToggle(MenuId::Wifi));
    let before = (w.var("wifi"), w.windows());

    w.feed(eww_state_core::Event::EwwReset);
    assert_eq!((w.var("wifi"), w.windows()), before);
}
