//! A fake world to drive the state machine against.
//!
//! The machine does no IO, so a test can stand in for the entire runtime: play
//! the part of nmcli, wpctl and the compositor, answer the actions the machine
//! dispatches, and then assert on exactly what the user would see. No eww, no
//! processes, no sleeping — and no clock either: time is a field here, moved by
//! the test, and sent in as the same tick the daemon would have sent.

#![allow(dead_code)] // a harness offers more than any one test uses

use std::collections::BTreeMap;
use std::time::Duration;
use std::time::Instant;

use eww_state_core::Action;
use eww_state_core::ActionFile;
use eww_state_core::ActionProbe;
use eww_state_core::ActionShell;
use eww_state_core::Audio;
use eww_state_core::Battery;
use eww_state_core::BtSnapshot;
use eww_state_core::Config;
use eww_state_core::DockApp;
use eww_state_core::Event;
use eww_state_core::EventClock;
use eww_state_core::EventJob;
use eww_state_core::EventProbe;
use eww_state_core::EventUi;
use eww_state_core::JobId;
use eww_state_core::MenuId;
use eww_state_core::NetSnapshot;
use eww_state_core::Probe;
use eww_state_core::Shot;
use eww_state_core::StateBar;
use eww_state_core::Theme;

pub struct World {
    pub bar: StateBar,
    now: Instant,

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
    capturing: Option<JobId>,

    /// Everything that actually happened, in order.
    pub log: Vec<String>,
}

impl World {
    pub fn new() -> World {
        let now = Instant::now();
        World {
            bar: StateBar::new(Config::new("/icons", "/cache"), now),
            now,
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
        let mut queue = Vec::new();
        self.bar.handle_events(events, &mut queue);
        self.drain(queue);
        self
    }

    pub fn ui(&mut self, event: EventUi) -> &mut World {
        self.feed(&[Event::FromUi(event)])
    }

    pub fn menu(&mut self, menu: MenuId) -> &mut World {
        self.ui(EventUi::MenuToggled { menu })
    }

    /// Move the clock and tell the machine, the way the daemon's tick does.
    pub fn wait(&mut self, d: Duration) -> &mut World {
        self.now += d;
        let now = self.now;
        self.feed(&[Event::FromClock(EventClock::Tick { now })])
    }

    /// Answer everything that was held while `hold` was set, in the order it
    /// was dispatched — the way a slow command finally returning would.
    pub fn release(&mut self) -> &mut World {
        self.hold = false;
        let held = std::mem::take(&mut self.held);
        self.drain(held);
        self
    }

    /// Deliver a probe result the machine never asked for — a poll that was
    /// already in flight when something else changed.
    pub fn stale_sample(&mut self, observed: EventProbe) -> &mut World {
        self.feed(&[Event::FromProbe(observed)])
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
                next.extend(self.perform(action));
            }
            queue = next;
        }
    }

    fn perform(&mut self, action: Action) -> Vec<Action> {
        self.log.push(describe(&action));
        let id = action.id();
        let (ok, detail) = match action {
            Action::ToProbe {
                action: ActionProbe::Sample { what },
                ..
            } => {
                let observed = self.sample(what);
                let mut out = Vec::new();
                self.bar
                    .handle_events(&[Event::FromProbe(observed)], &mut out);
                self.bar.handle_events(&[finished(id, true)], &mut out);
                return out;
            }
            // A capture is answered by the test calling `capture_finished`,
            // because the user is holding the mouse in the middle of it.
            Action::ToShell {
                action: ActionShell::Capture,
                ..
            } => {
                self.capturing = Some(id);
                return Vec::new();
            }
            Action::ToShell { action, .. } => self.shell(action),
            Action::ToFile { action, .. } => self.file(action),
        };
        let mut out = Vec::new();
        self.bar.handle_events(&[job(id, ok, detail)], &mut out);
        out
    }

    fn shell(&mut self, action: ActionShell) -> (bool, Option<String>) {
        match action {
            ActionShell::SetVolume { percent } => {
                self.volume = percent;
                (true, None)
            }
            ActionShell::SetBrightness { percent } => {
                self.brightness = percent;
                (true, None)
            }
            ActionShell::WifiSetEnabled { on } => {
                self.net.enabled = on;
                if !on {
                    self.net.connected = None;
                    self.net.networks.iter_mut().for_each(|n| n.active = false);
                }
                (true, None)
            }
            ActionShell::WifiScan => (true, None),
            ActionShell::WifiConnect { ssid } => {
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
            ActionShell::BtSetPowered { on } => {
                self.bt.powered = on;
                (true, None)
            }
            ActionShell::BtScan => (true, None),
            ActionShell::BtConnect { mac, .. } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = true;
                    }
                }
                (true, None)
            }
            ActionShell::BtDisconnect { mac, .. } => {
                for d in &mut self.bt.devices {
                    if d.mac == mac {
                        d.connected = false;
                    }
                }
                (true, None)
            }
            ActionShell::Launch { .. } => (true, None),
            ActionShell::Capture => unreachable!("captures are held above"),
            ActionShell::CopyShot { .. } | ActionShell::Notify { .. } => (true, None),
        }
    }

    fn file(&mut self, action: ActionFile) -> (bool, Option<String>) {
        match action {
            ActionFile::WriteDock { apps } => {
                self.dock = apps;
                (true, None)
            }
            ActionFile::RenderBattery { .. }
            | ActionFile::SaveShot { .. }
            | ActionFile::DeleteShot { .. } => (true, None),
        }
    }

    fn sample(&self, probe: Probe) -> EventProbe {
        match probe {
            Probe::Date => EventProbe::Date {
                text: "Mon Aug 12  14:30".into(),
            },
            Probe::Battery => EventProbe::Battery {
                battery: Battery {
                    percent: 80,
                    charging: false,
                },
            },
            Probe::Theme => EventProbe::Theme { theme: Theme::Dark },
            Probe::Audio => EventProbe::Audio {
                audio: Audio {
                    volume: self.volume,
                    muted: self.muted,
                },
            },
            Probe::Backlight => EventProbe::Backlight {
                percent: self.brightness,
            },
            Probe::Net => EventProbe::Net {
                net: self.net.clone(),
            },
            Probe::Bt => EventProbe::Bt {
                bt: self.bt.clone(),
            },
            Probe::Dock => EventProbe::Dock {
                apps: self.dock.clone(),
            },
            Probe::Fullscreen => EventProbe::Fullscreen {
                on: self.fullscreen,
            },
        }
    }

    /// The user finished dragging out a region.
    pub fn capture_finished(&mut self, path: &str, width: u32, height: u32) -> &mut World {
        self.feed(&[Event::FromJob(EventJob::ShotStaged {
            shot: Shot {
                path: path.into(),
                width,
                height,
            },
        })]);
        if let Some(id) = self.capturing.take() {
            self.feed(&[finished(id, true)]);
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

fn finished(id: JobId, ok: bool) -> Event {
    job(id, ok, None)
}

fn job(id: JobId, ok: bool, detail: Option<String>) -> Event {
    Event::FromJob(EventJob::Finished { id, ok, detail })
}

fn describe(action: &Action) -> String {
    match action {
        Action::ToProbe {
            action: ActionProbe::Sample { what },
            ..
        } => format!("probe {}", what.as_str()),
        Action::ToShell { action, .. } => match action {
            ActionShell::SetVolume { percent } => format!("set-volume {percent}"),
            ActionShell::SetBrightness { percent } => format!("set-brightness {percent}"),
            ActionShell::WifiSetEnabled { on } => format!("wifi-radio {on}"),
            ActionShell::WifiScan => "wifi-scan".into(),
            ActionShell::WifiConnect { ssid } => format!("wifi-connect {ssid}"),
            ActionShell::BtSetPowered { on } => format!("bt-power {on}"),
            ActionShell::BtScan => "bt-scan".into(),
            ActionShell::BtConnect { mac, .. } => format!("bt-connect {mac}"),
            ActionShell::BtDisconnect { mac, .. } => format!("bt-disconnect {mac}"),
            ActionShell::Launch { cmd } => format!("launch {cmd}"),
            ActionShell::Capture => "shot-capture".into(),
            ActionShell::CopyShot { path } => format!("shot-copy {path}"),
            ActionShell::Notify { body, .. } => format!("notify {body}"),
        },
        Action::ToFile { action, .. } => match action {
            ActionFile::WriteDock { apps } => format!(
                "persist-dock {}",
                apps.iter()
                    .map(|a| a.id.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            ActionFile::RenderBattery { percent, charging } => {
                format!("render-battery {percent} {charging}")
            }
            ActionFile::SaveShot { path } => format!("shot-save {path}"),
            ActionFile::DeleteShot { path } => format!("shot-delete {path}"),
        },
    }
}
