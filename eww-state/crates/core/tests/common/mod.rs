//! A fake world to drive the state machine against.
//!
//! The machine does no IO, so a test can stand in for the entire shell: play
//! the part of nmcli, wpctl and the compositor, answer the actions the machine
//! dispatches, and then assert on exactly what the user would see. No eww, no
//! processes, no sleeping.

#![allow(dead_code)] // a harness offers more than any one test uses

use std::collections::BTreeMap;

use eww_state_core::{
    Action, ActionKind, Bar, BtSnapshot, Config, DockApp, Event, Intent, NetSnapshot, Obs, Probe,
    Shot, Theme,
};

pub struct World {
    pub bar: Bar,

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
    capturing: Option<eww_state_core::JobId>,

    /// Everything that actually happened, in order.
    pub log: Vec<String>,
}

impl World {
    pub fn new() -> World {
        World {
            bar: Bar::new(Config::new("/icons", "/cache")),
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

    /// Feed an event in and settle: run every action it produced, and every
    /// action those produced, until nothing is left. This is the shell's whole
    /// job, minus the waiting.
    pub fn feed(&mut self, ev: Event) -> &mut World {
        let mut queue = self.bar.handle(ev);
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
                next.extend(self.perform(action));
            }
            queue = next;
        }
        self
    }

    pub fn intent(&mut self, i: Intent) -> &mut World {
        self.feed(Event::Intent(i))
    }

    /// Answer everything that was held while `hold` was set, in the order it
    /// was dispatched — the way a slow command finally returning would.
    pub fn release(&mut self) -> &mut World {
        self.hold = false;
        let held = std::mem::take(&mut self.held);
        let mut queue: Vec<Action> = held;
        let mut guard = 0;
        while !queue.is_empty() {
            guard += 1;
            assert!(guard < 100, "actions did not settle");
            let mut next = Vec::new();
            for action in queue {
                next.extend(self.perform(action));
            }
            queue = next;
        }
        self
    }

    /// Deliver a probe result the machine never asked for — a poll that was
    /// already in flight when something else changed.
    pub fn stale_sample(&mut self, obs: Obs) -> &mut World {
        self.feed(Event::Observed(obs))
    }

    fn perform(&mut self, action: Action) -> Vec<Action> {
        let Action { id, kind } = action;
        self.log.push(describe(&kind));
        let (ok, detail) = match kind {
            ActionKind::Probe(p) => {
                let obs = self.sample(p);
                let mut out = self.bar.handle(Event::Observed(obs));
                out.extend(self.bar.handle(Event::JobDone {
                    id,
                    ok: true,
                    detail: None,
                }));
                return out;
            }
            ActionKind::SetVolume(v) => {
                self.volume = v;
                (true, None)
            }
            ActionKind::SetBrightness(v) => {
                self.brightness = v;
                (true, None)
            }
            ActionKind::WifiSetEnabled(on) => {
                self.net.enabled = on;
                if !on {
                    self.net.connected = None;
                    self.net.networks.iter_mut().for_each(|n| n.active = false);
                }
                (true, None)
            }
            ActionKind::WifiScan => (true, None),
            ActionKind::WifiConnect { ssid } => {
                if self.wifi_fails {
                    (false, None)
                } else {
                    self.net.connected = Some(ssid.clone());
                    for n in &mut self.net.networks {
                        n.active = n.ssid == ssid;
                    }
                    (true, None)
                }
            }
            ActionKind::BtSetPowered(on) => {
                self.bt.powered = on;
                (true, None)
            }
            ActionKind::BtScan => (true, None),
            ActionKind::BtConnect { mac, .. } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = true;
                    }
                }
                (true, None)
            }
            ActionKind::BtDisconnect { mac, .. } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = false;
                    }
                }
                (true, None)
            }
            ActionKind::Launch { .. } => (true, None),
            ActionKind::PersistDock(apps) => {
                self.dock = apps;
                (true, None)
            }
            ActionKind::RenderBattery { .. } => (true, None),
            // A capture is answered by the test calling `capture_finished`,
            // because the user is holding the mouse in the middle of it.
            ActionKind::ShotCapture => {
                self.capturing = Some(id);
                return Vec::new();
            }
            ActionKind::ShotCopy { .. }
            | ActionKind::ShotSave { .. }
            | ActionKind::ShotDelete { .. }
            | ActionKind::Notify { .. } => (true, None),
        };
        self.bar.handle(Event::JobDone { id, ok, detail })
    }

    fn sample(&self, probe: Probe) -> Obs {
        match probe {
            Probe::Clock => Obs::Clock("Mon Aug 12  14:30".into()),
            Probe::Battery => Obs::Battery(eww_state_core::Battery {
                percent: 80,
                charging: false,
            }),
            Probe::Theme => Obs::Theme(Theme::Dark),
            Probe::Audio => Obs::Audio(eww_state_core::Audio {
                volume: self.volume,
                muted: self.muted,
            }),
            Probe::Backlight => Obs::Backlight(self.brightness),
            Probe::Net => Obs::Net(self.net.clone()),
            Probe::Bt => Obs::Bt(self.bt.clone()),
            Probe::Dock => Obs::Dock(self.dock.clone()),
            Probe::Fullscreen => Obs::Fullscreen(self.fullscreen),
        }
    }

    /// The user finished dragging out a region.
    pub fn capture_finished(&mut self, path: &str, w: u32, h: u32) -> &mut World {
        self.intent(Intent::ShotStage(Shot {
            path: path.into(),
            width: w,
            height: h,
        }));
        if let Some(id) = self.capturing.take() {
            self.feed(Event::JobDone {
                id,
                ok: true,
                detail: None,
            });
        }
        self
    }

    // ------------------------------------------------------------ assertions

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
        let bar_menus: Vec<String> = eww_state_core::MenuId::ALL
            .iter()
            .map(|m| m.window())
            .collect();
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

fn describe(kind: &ActionKind) -> String {
    match kind {
        ActionKind::Probe(p) => format!("probe {}", p.as_str()),
        ActionKind::SetVolume(v) => format!("set-volume {v}"),
        ActionKind::SetBrightness(v) => format!("set-brightness {v}"),
        ActionKind::WifiSetEnabled(on) => format!("wifi-radio {on}"),
        ActionKind::WifiScan => "wifi-scan".into(),
        ActionKind::WifiConnect { ssid } => format!("wifi-connect {ssid}"),
        ActionKind::BtSetPowered(on) => format!("bt-power {on}"),
        ActionKind::BtScan => "bt-scan".into(),
        ActionKind::BtConnect { mac, .. } => format!("bt-connect {mac}"),
        ActionKind::BtDisconnect { mac, .. } => format!("bt-disconnect {mac}"),
        ActionKind::Launch { cmd } => format!("launch {cmd}"),
        ActionKind::PersistDock(apps) => format!(
            "persist-dock {}",
            apps.iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ),
        ActionKind::RenderBattery { percent, charging } => {
            format!("render-battery {percent} {charging}")
        }
        ActionKind::ShotCapture => "shot-capture".into(),
        ActionKind::ShotCopy { path } => format!("shot-copy {path}"),
        ActionKind::ShotSave { path } => format!("shot-save {path}"),
        ActionKind::ShotDelete { path } => format!("shot-delete {path}"),
        ActionKind::Notify { body, .. } => format!("notify {body}"),
    }
}
