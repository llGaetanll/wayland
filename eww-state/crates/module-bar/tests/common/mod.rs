//! A fake world to drive the bar against.
//!
//! No module does IO here, so a test can stand in for the entire runtime: play
//! the part of nmcli, wpctl and the compositor, answer the actions the machine
//! dispatches, and then assert on exactly what the user would see. No eww, no
//! processes, no sleeping — and no clock either: time is a field the test moves,
//! sent in as the same tick the daemon would have sent.

#![allow(dead_code)] // a harness offers more than any one test uses

use std::collections::BTreeMap;

use api::Millis;
use module_audio::Audio;
use module_audio::EventAudio;
use module_backlight::EventBacklight;
use module_bar::Action;
use module_bar::ConfigBar;
use module_bar::Event;
use module_bar::EventBar;
use module_bar::EventClock;
use module_bar::MenuId;
use module_bar::StateBar;
use module_battery::Battery;
use module_battery::EventBattery;
use module_bluetooth::BtSnapshot;
use module_bluetooth::EventBluetooth;
use module_date::EventDate;
use module_dock::DockApp;
use module_dock::EventDock;
use module_shot::EventShot;
use module_shot::Shot;
use module_theme::EventTheme;
use module_theme::Theme;
use module_wifi::EventWifi;
use module_wifi::NetSnapshot;

pub struct World {
    pub bar: StateBar,
    now: Millis,

    // What the machine can observe about the world.
    pub volume: u8,
    pub muted: bool,
    pub brightness: u8,
    pub net: NetSnapshot,
    pub bt: BtSnapshot,
    pub dock: Vec<DockApp>,
    pub fullscreen: bool,

    /// Joining a network fails until this is cleared.
    pub wifi_fails: bool,
    /// Actions are answered immediately unless they are held here first.
    held: Vec<Action>,
    pub hold: bool,
    /// The capture the user is currently dragging out a region for.
    capturing: Option<api::JobId>,

    /// Everything that actually happened, in order.
    pub log: Vec<String>,
}

impl World {
    pub fn new() -> World {
        World {
            bar: StateBar::new(ConfigBar::new("/icons", "/cache")),
            now: Millis::default(),
            volume: 50,
            muted: false,
            brightness: 40,
            net: NetSnapshot::default(),
            bt: BtSnapshot::default(),
            dock: Vec::new(),
            fullscreen: false,
            wifi_fails: false,
            held: Vec::new(),
            hold: false,
            capturing: None,
            log: Vec::new(),
        }
    }

    /// Feed events in and settle: run every action they produced, and every
    /// action those produced, until nothing is left. This is the runtime's
    /// whole job, minus the waiting.
    pub fn feed(&mut self, events: &[Event]) -> &mut World {
        self.bar.handle_events(events);
        let mut queue = Vec::new();
        self.bar.emit_actions(&mut queue);
        self.drain(queue);
        self
    }

    pub fn menu(&mut self, menu: MenuId) -> &mut World {
        self.feed(&[Event::Bar(EventBar::MenuToggled { menu })])
    }

    pub fn close_menu(&mut self) -> &mut World {
        self.feed(&[Event::Bar(EventBar::MenuDismissed)])
    }

    /// Move the clock and tell the machine, the way the daemon's tick does.
    pub fn wait(&mut self, d: Millis) -> &mut World {
        self.now = self.now + d;
        let now = self.now;
        self.feed(&[Event::Clock(EventClock::Tick { now })])
    }

    /// Answer everything that was held while `hold` was set, in the order it
    /// was dispatched — the way a slow command finally returning would.
    pub fn release(&mut self) -> &mut World {
        self.hold = false;
        let held = std::mem::take(&mut self.held);
        self.drain(held);
        self
    }

    /// Deliver a sample the machine never asked for — a poll that was already
    /// in flight when something else changed.
    pub fn stale_sample(&mut self, event: Event) -> &mut World {
        self.feed(&[event])
    }

    fn drain(&mut self, mut queue: Vec<Action>) {
        let mut guard = 0;
        while !queue.is_empty() {
            guard += 1;
            assert!(guard < 100, "actions did not settle: {queue:?}");
            let mut next = Vec::new();
            for action in queue {
                if self.hold {
                    self.held.push(action);
                    continue;
                }
                let events = self.perform(action);
                self.bar.handle_events(&events);
                self.bar.emit_actions(&mut next);
            }
            queue = next;
        }
    }

    fn perform(&mut self, action: Action) -> Vec<Event> {
        self.log.push(describe(&action));
        match action {
            Action::SampleFullscreen => vec![Event::Bar(EventBar::Fullscreen {
                on: self.fullscreen,
            })],
            Action::Audio(a) => self.audio(a),
            Action::Backlight(a) => self.backlight(a),
            Action::Battery(a) => self.battery(a),
            Action::Bluetooth(a) => self.bluetooth(a),
            Action::Date(_) => vec![Event::Date(EventDate::Observed {
                text: "Mon Aug 12  14:30".into(),
            })],
            Action::Dock(a) => self.dock_action(a),
            Action::Shot(a) => self.shot(a),
            Action::Theme(_) => vec![Event::Theme(EventTheme::Observed { theme: Theme::Dark })],
            Action::Wifi(a) => self.wifi(a),
        }
    }

    fn audio(&mut self, action: module_audio::ActionAudio) -> Vec<Event> {
        use module_audio::ActionAudio;
        match action {
            ActionAudio::Sample => vec![Event::Audio(EventAudio::Observed {
                audio: Audio {
                    volume: self.volume,
                    muted: self.muted,
                },
            })],
            ActionAudio::Set { percent } => {
                self.volume = percent;
                vec![Event::Audio(EventAudio::Finished)]
            }
        }
    }

    fn backlight(&mut self, action: module_backlight::ActionBacklight) -> Vec<Event> {
        use module_backlight::ActionBacklight;
        match action {
            ActionBacklight::Sample => vec![Event::Backlight(EventBacklight::Observed {
                percent: self.brightness,
            })],
            ActionBacklight::Set { percent } => {
                self.brightness = percent;
                vec![Event::Backlight(EventBacklight::Finished)]
            }
        }
    }

    fn battery(&mut self, action: module_battery::ActionBattery) -> Vec<Event> {
        use module_battery::ActionBattery;
        match action {
            ActionBattery::Sample => vec![Event::Battery(EventBattery::Observed {
                battery: Battery {
                    percent: 80,
                    charging: false,
                },
            })],
            ActionBattery::Render { id, .. } => {
                vec![Event::Battery(EventBattery::Finished { id, ok: true })]
            }
        }
    }

    fn bluetooth(&mut self, action: module_bluetooth::ActionBluetooth) -> Vec<Event> {
        use module_bluetooth::ActionBluetooth;
        match action {
            ActionBluetooth::Sample => vec![Event::Bluetooth(EventBluetooth::Observed {
                bt: self.bt.clone(),
            })],
            ActionBluetooth::Scan | ActionBluetooth::Notify { .. } => Vec::new(),
            ActionBluetooth::SetPowered { id, on } => {
                self.bt.powered = on;
                vec![Event::Bluetooth(EventBluetooth::Finished { id, ok: true })]
            }
            ActionBluetooth::Connect { id, mac } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = true;
                    }
                }
                vec![Event::Bluetooth(EventBluetooth::Finished { id, ok: true })]
            }
            ActionBluetooth::Disconnect { id, mac } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = false;
                    }
                }
                vec![Event::Bluetooth(EventBluetooth::Finished { id, ok: true })]
            }
        }
    }

    fn dock_action(&mut self, action: module_dock::ActionDock) -> Vec<Event> {
        use module_dock::ActionDock;
        match action {
            ActionDock::Sample => vec![Event::Dock(EventDock::Observed {
                apps: self.dock.clone(),
            })],
            ActionDock::Launch { .. } => Vec::new(),
            ActionDock::Write { apps } => {
                self.dock = apps;
                Vec::new()
            }
        }
    }

    fn shot(&mut self, action: module_shot::ActionShot) -> Vec<Event> {
        use module_shot::ActionShot;
        match action {
            // Answered by the test calling `capture_finished`, because the user
            // is holding the mouse in the middle of it.
            ActionShot::Capture { id } => {
                self.capturing = Some(id);
                Vec::new()
            }
            ActionShot::Copy { .. } | ActionShot::Save { .. } | ActionShot::Delete { .. } => {
                Vec::new()
            }
        }
    }

    fn wifi(&mut self, action: module_wifi::ActionWifi) -> Vec<Event> {
        use module_wifi::ActionWifi;
        match action {
            ActionWifi::Sample => vec![Event::Wifi(EventWifi::Observed {
                net: self.net.clone(),
            })],
            ActionWifi::Scan | ActionWifi::Notify { .. } => Vec::new(),
            ActionWifi::SetEnabled { id, on } => {
                self.net.enabled = on;
                if !on {
                    self.net.connected = None;
                    self.net.networks.iter_mut().for_each(|n| n.active = false);
                }
                vec![Event::Wifi(EventWifi::Finished {
                    id,
                    ok: true,
                    detail: None,
                })]
            }
            ActionWifi::Connect { id, ssid } => {
                let ok = !self.wifi_fails;
                if ok {
                    self.net.connected = Some(ssid.clone());
                    for n in &mut self.net.networks {
                        n.active = n.ssid == ssid;
                    }
                }
                vec![Event::Wifi(EventWifi::Finished {
                    id,
                    ok,
                    detail: None,
                })]
            }
        }
    }

    /// The user finished dragging out a region.
    pub fn capture_finished(&mut self, path: &str, width: u32, height: u32) -> &mut World {
        self.feed(&[Event::Shot(EventShot::Staged {
            shot: Shot {
                path: path.into(),
                width,
                height,
            },
        })]);
        if let Some(id) = self.capturing.take() {
            self.feed(&[Event::Shot(EventShot::Finished { id })]);
        }
        self
    }

    pub fn windows(&self) -> Vec<String> {
        self.bar.windows().into_iter().map(|w| w.name()).collect()
    }

    pub fn var(&self, name: &str) -> String {
        self.bar
            .vars()
            .get(name)
            .unwrap_or_else(|| panic!("no such var: {name}"))
            .to_string()
    }

    /// Bar dropdowns only — the screenshot preview is not one of them.
    pub fn menus_open(&self) -> Vec<String> {
        let bar_menus: Vec<String> = MenuId::ALL.iter().map(|m| m.window()).collect();
        self.windows()
            .into_iter()
            .filter(|w| bar_menus.contains(w))
            .collect()
    }

    pub fn notifications(&self) -> Vec<&str> {
        self.log
            .iter()
            .filter_map(|l| l.strip_prefix("notify "))
            .collect()
    }

    pub fn counts(&self) -> BTreeMap<&str, usize> {
        let mut counts = BTreeMap::new();
        for line in &self.log {
            let head = line.split(' ').next().unwrap_or(line);
            *counts.entry(head).or_insert(0) += 1;
        }
        counts
    }
}

fn describe(action: &Action) -> String {
    use module_bluetooth::ActionBluetooth as Bt;
    use module_dock::ActionDock as Dock;
    use module_shot::ActionShot as Shot;
    use module_wifi::ActionWifi as Wifi;
    match action {
        Action::SampleFullscreen => "sample fullscreen".into(),
        Action::Audio(a) => format!("audio {a:?}"),
        Action::Backlight(a) => format!("backlight {a:?}"),
        Action::Battery(a) => format!("battery {a:?}"),
        Action::Date(_) => "sample date".into(),
        Action::Theme(_) => "sample theme".into(),
        Action::Bluetooth(Bt::Notify { body }) => format!("notify {body}"),
        Action::Bluetooth(a) => format!("bt {a:?}"),
        Action::Dock(Dock::Launch { cmd }) => format!("launch {cmd}"),
        Action::Dock(Dock::Write { apps }) => format!(
            "persist-dock {}",
            apps.iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
        Action::Dock(a) => format!("dock {a:?}"),
        Action::Shot(Shot::Capture { .. }) => "shot-capture".into(),
        Action::Shot(Shot::Save { path }) => format!("shot-save {path}"),
        Action::Shot(a) => format!("shot {a:?}"),
        Action::Wifi(Wifi::Notify { body }) => format!("notify {body}"),
        Action::Wifi(Wifi::Scan) => "wifi-scan".into(),
        Action::Wifi(a) => format!("wifi {a:?}"),
    }
}
