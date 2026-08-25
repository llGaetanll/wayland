//! End-to-end scenarios, played against the fake world in `common`.
//!
//! Each of these is a bug the shell-script version actually had, written down
//! as the behaviour that replaces it.

mod common;

use api::Millis;
use common::World;
use module_audio::Audio;
use module_audio::EventAudio;
use module_bar::Event;
use module_bar::EventBar;
use module_bar::MenuId;
use module_bar::ModuleId;
use module_bluetooth::BtDevice;
use module_bluetooth::BtSnapshot;
use module_bluetooth::EventBluetooth;
use module_shot::EventShot;
use module_shot::ShotAction;
use module_wifi::EventWifi;
use module_wifi::NetSnapshot;
use module_wifi::WifiNetwork;

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
        w.menu(menu);
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
    w.menu(MenuId::Wifi);
    assert_eq!(w.menus_open(), ["wifi-menu"]);
    w.menu(MenuId::Bt);
    assert_eq!(w.menus_open(), ["bt-menu"]);
    assert!(w.windows().contains(&"menu-backdrop".to_string()));

    // Clicking outside dismisses whatever is up.
    w.close_menu();
    assert!(w.menus_open().is_empty());
    assert!(!w.windows().contains(&"menu-backdrop".to_string()));
}

/// A drag fires many times a second while a 2s poll is also running. The slider
/// used to snap back to a value from before the drag.
#[test]
fn dragging_the_volume_slider_beats_the_poll() {
    let mut w = World::new();
    w.menu(MenuId::Volume);

    w.hold = true; // the setter has not returned yet
    for percent in [55, 60, 65, 70] {
        w.feed(&[Event::Audio(EventAudio::Set { percent })]);
    }
    // A poll from before the drag lands in the middle of it.
    w.stale_sample(Event::Audio(EventAudio::Observed {
        audio: Audio {
            volume: 50,
            muted: false,
        },
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
    w.feed(&[ModuleId::Wifi.due()]);
    w.menu(MenuId::Wifi);

    w.hold = true;
    w.feed(&[Event::Wifi(EventWifi::Picked {
        ssid: "home".into(),
    })]);
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
    w.feed(&[ModuleId::Wifi.due()]);
    w.menu(MenuId::Wifi);
    w.feed(&[Event::Wifi(EventWifi::Picked {
        ssid: "cafe".into(),
    })]);

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
    w.feed(&[ModuleId::Wifi.due()]);
    assert!(w.var("wifi").contains("\"enabled\":true"));

    w.hold = true;
    w.feed(&[Event::Wifi(EventWifi::Toggled)]);
    assert!(
        w.var("wifi").contains("\"enabled\":false"),
        "switch flips at once"
    );
    w.stale_sample(Event::Wifi(EventWifi::Observed {
        net: networks(&[("home", 80, true)]),
    }));
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
    w.feed(&[ModuleId::Bluetooth.due()]);
    w.menu(MenuId::Bt);

    w.hold = true;
    w.feed(&[Event::Bluetooth(EventBluetooth::Picked {
        mac: "AA:BB".into(),
    })]);
    assert_eq!(w.var("bt_connecting"), "AA:BB");
    w.release();

    assert_eq!(w.menus_open(), ["bt-menu"]);
    assert_eq!(w.var("bt_connecting"), "");
    assert_eq!(w.notifications(), ["Connected Buds"]);
    assert!(w.var("bt_icon").ends_with("bluetooth-paired-symbolic.svg"));

    // Clicking the connected device again disconnects it.
    w.feed(&[Event::Bluetooth(EventBluetooth::Picked {
        mac: "AA:BB".into(),
    })]);
    assert_eq!(w.notifications().last(), Some(&"Disconnected Buds"));
    assert!(w.var("bt_icon").ends_with("bluetooth-active-symbolic.svg"));
}

/// A Print press must not disturb an open bar dropdown, in either direction.
#[test]
fn the_screenshot_preview_and_the_bar_menus_ignore_each_other() {
    let mut w = World::new();
    w.menu(MenuId::Wifi);

    w.feed(&[Event::Shot(EventShot::Requested)]);
    w.capture_finished("/shots/a.png", 1600, 900);
    assert_eq!(w.menus_open(), ["wifi-menu"], "the bar menu is untouched");
    assert!(w.windows().contains(&"shot-menu".to_string()));
    assert_eq!(w.var("shot_w"), "320", "preview is fitted, not stretched");
    assert_eq!(w.var("shot_h"), "180");

    // Clicking outside the preview dismisses only the preview.
    w.feed(&[Event::Shot(EventShot::Acted {
        action: ShotAction::Dismiss,
    })]);
    assert!(!w.windows().contains(&"shot-menu".to_string()));
    assert_eq!(w.menus_open(), ["wifi-menu"]);

    // And dismissing the bar menu leaves no screenshot windows behind.
    w.close_menu();
    assert!(w.windows().iter().all(|n| n == "bar" || n == "dock"));
}

/// Mashing Print used to stack region selectors and overwrite the staged file.
#[test]
fn a_second_print_during_selection_is_ignored() {
    let mut w = World::new();
    w.feed(&[Event::Shot(EventShot::Requested)]);
    w.feed(&[Event::Shot(EventShot::Requested)]);
    w.feed(&[Event::Shot(EventShot::Requested)]);
    assert_eq!(w.counts().get("shot-capture"), Some(&1));

    w.capture_finished("/shots/a.png", 800, 600);
    w.feed(&[Event::Shot(EventShot::Acted {
        action: ShotAction::Save,
    })]);
    assert!(w.log.contains(&"shot-save /shots/a.png".to_string()));

    // With the first one finished, Print works again.
    w.feed(&[Event::Shot(EventShot::Requested)]);
    assert_eq!(w.counts().get("shot-capture"), Some(&2));
}

/// A region selection nobody finishes used to wedge Print for the rest of the
/// session: only one capture may be in flight, and nothing ever cleared it.
#[test]
fn an_abandoned_selection_stops_blocking_the_key() {
    let mut w = World::new();
    w.feed(&[Event::Shot(EventShot::Requested)]);
    w.feed(&[Event::Shot(EventShot::Requested)]);
    assert_eq!(w.counts().get("shot-capture"), Some(&1));

    let timeout = module_shot::ConfigShot::default().timeout;
    w.wait(timeout.since(Millis::secs(1)));
    w.feed(&[Event::Shot(EventShot::Requested)]);
    assert_eq!(
        w.counts().get("shot-capture"),
        Some(&1),
        "still waiting on the user, not stuck"
    );

    w.wait(Millis::secs(1));
    w.feed(&[Event::Shot(EventShot::Requested)]);
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
        w.feed(&[Event::Bar(EventBar::FullscreenDue)]);
        assert!(w.windows().is_empty(), "both hide together");

        w.fullscreen = false;
        w.feed(&[Event::Bar(EventBar::FullscreenDue)]);
        assert_eq!(w.windows(), ["bar", "dock"], "and both come back");
    }
}

/// Opening a menu should cost a refresh; leaving it open should cost the faster
/// poll only while it is up.
#[test]
fn polling_follows_what_is_on_screen() {
    let mut w = World::new();
    let idle = w.bar.polls()[&ModuleId::Wifi];

    w.menu(MenuId::Wifi);
    let open = w.bar.polls()[&ModuleId::Wifi];
    assert!(open < idle, "an open menu polls faster");
    assert!(
        w.log.contains(&"wifi-scan".to_string()),
        "and rescans on open"
    );

    w.close_menu();
    assert_eq!(w.bar.polls()[&ModuleId::Wifi], idle);
}

/// eww restarting must not lose anything: the machine holds the truth and the
/// runtime republishes it.
#[test]
fn state_survives_an_eww_restart() {
    let mut w = World::new();
    w.net = networks(&[("home", 80, true)]);
    w.feed(&[ModuleId::Wifi.due()]);
    w.menu(MenuId::Wifi);
    let before = (w.var("wifi"), w.windows());

    w.feed(&[Event::Bar(EventBar::EwwRestarted)]);
    assert_eq!((w.var("wifi"), w.windows()), before);
}
