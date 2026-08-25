//! The Bluetooth state machine. No IO.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::millis::Millis;
use api::module::Module;
use api::util::b64;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionBluetooth;
use crate::config::ConfigBluetooth;
use crate::event::EventBluetooth;
use crate::model::BtSnapshot;

#[derive(Clone, Debug, Default)]
pub struct StateBluetooth {
    cfg: ConfigBluetooth,
    jobs: Jobs,
    bt: BtSnapshot,
    /// The device being connected or disconnected.
    busy: Option<Busy>,
    /// A power on/off is in flight; samples taken while it settles would report
    /// the old state and flip the switch back under the user's finger.
    toggling: Option<Inflight>,
    menu_open: bool,
    now: Millis,

    /// What the batch asked for and has not been sent yet.
    want_power: Option<bool>,
    want_device: Option<String>,
    want_scan: bool,
    want_sample: bool,
    say: Option<String>,
}

#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

#[derive(Clone, Debug)]
struct Busy {
    id: JobId,
    mac: String,
    /// The name as it was when the click happened, so a device that vanishes
    /// mid-connect is still named in the notification.
    name: String,
    disconnecting: bool,
    since: Millis,
}

impl StateBluetooth {
    pub fn new(cfg: ConfigBluetooth) -> StateBluetooth {
        StateBluetooth {
            cfg,
            ..StateBluetooth::default()
        }
    }

    pub fn snapshot(&self) -> &BtSnapshot {
        &self.bt
    }

    /// The MAC being acted on, if one is.
    pub fn acting_on(&self) -> Option<&str> {
        self.busy.as_ref().map(|b| b.mac.as_str())
    }

    /// A power toggle is out and has not been answered.
    pub fn settling(&self) -> bool {
        self.toggling.is_some()
    }

    pub fn icon(&self) -> String {
        self.cfg.icon(if !self.bt.powered {
            "bluetooth-disabled-symbolic"
        } else if self.bt.devices.iter().any(|d| d.connected) {
            "bluetooth-paired-symbolic"
        } else {
            "bluetooth-active-symbolic"
        })
    }

    fn subtitle(&self) -> &str {
        if !self.bt.powered {
            "Off"
        } else {
            self.bt.connected_name().unwrap_or("On")
        }
    }

    /// Give up on a job that never reported back, so a switch cannot stay stuck
    /// mid-flip and a row cannot say "Connecting…" for the rest of the session.
    fn expire(&mut self) {
        let timeout = self.cfg.job_timeout;
        if self
            .toggling
            .is_some_and(|t| self.now.since(t.since) >= timeout)
        {
            self.toggling = None;
            self.want_sample = true;
        }
        if self
            .busy
            .as_ref()
            .is_some_and(|b| self.now.since(b.since) >= timeout)
        {
            let Some(busy) = self.busy.take() else {
                unreachable!("the branch above just matched on it")
            };
            let verb = if busy.disconnecting {
                "disconnect"
            } else {
                "connect"
            };
            self.say = Some(format!("Failed to {verb} {}", busy.name));
            self.want_sample = true;
        }
    }

    /// Read the list here rather than at click time, so a row that has since
    /// moved cannot send a command about a device that is no longer there.
    fn start_device(&mut self, actions: &mut Vec<ActionBluetooth>, mac: String) {
        if self.busy.is_some() {
            return;
        }
        let Some(dev) = self.bt.devices.iter().find(|d| d.mac == mac) else {
            return;
        };
        let (name, disconnecting) = (dev.name.clone(), dev.connected);
        let id = self.jobs.take();
        actions.push(if disconnecting {
            ActionBluetooth::Disconnect {
                id,
                mac: mac.clone(),
            }
        } else {
            ActionBluetooth::Connect {
                id,
                mac: mac.clone(),
            }
        });
        self.busy = Some(Busy {
            id,
            mac,
            name,
            disconnecting,
            since: self.now,
        });
    }
}

impl Module for StateBluetooth {
    type Event = EventBluetooth;
    type Action = ActionBluetooth;
    type Req = ();

    fn handle_event(&mut self, event: &EventBluetooth) {
        match event {
            EventBluetooth::Due => self.want_sample = true,
            EventBluetooth::Toggled => {
                let on = !self.bt.powered;
                self.bt.powered = on;
                if !on {
                    self.bt.devices.clear();
                }
                self.want_power = Some(on);
                self.want_scan = self.want_scan || on;
            }
            EventBluetooth::Picked { mac } => {
                if self.busy.is_none() && self.want_device.is_none() {
                    self.want_device = Some(mac.clone());
                }
            }
            EventBluetooth::Observed { bt } => {
                if self.toggling.is_none() && self.want_power.is_none() {
                    self.bt = bt.clone();
                }
            }
            EventBluetooth::MenuOpened => {
                self.menu_open = true;
                self.want_scan = self.want_scan || self.bt.powered;
                self.want_sample = true;
            }
            EventBluetooth::MenuClosed => self.menu_open = false,
            EventBluetooth::Tick { now } => {
                self.now = *now;
                self.expire();
            }
            EventBluetooth::Finished { id, ok } => {
                let id = *id;
                if self.toggling.is_some_and(|t| t.id == id) {
                    self.toggling = None;
                    self.want_sample = true;
                } else if self.busy.as_ref().is_some_and(|b| b.id == id) {
                    let Some(busy) = self.busy.take() else {
                        unreachable!("the branch above just matched on it")
                    };
                    self.say = Some(match (busy.disconnecting, ok) {
                        (true, true) => format!("Disconnected {}", busy.name),
                        (true, false) => format!("Failed to disconnect {}", busy.name),
                        (false, true) => format!("Connected {}", busy.name),
                        (false, false) => format!("Failed to connect {}", busy.name),
                    });
                    self.want_sample = true;
                }
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionBluetooth>) -> Option<()> {
        if let Some(on) = self.want_power.take() {
            let id = self.jobs.take();
            actions.push(ActionBluetooth::SetPowered { id, on });
            self.toggling = Some(Inflight {
                id,
                since: self.now,
            });
        }
        if std::mem::take(&mut self.want_scan) {
            actions.push(ActionBluetooth::Scan);
        }
        if let Some(mac) = self.want_device.take() {
            self.start_device(actions, mac);
        }
        if let Some(body) = self.say.take() {
            actions.push(ActionBluetooth::Notify { body });
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionBluetooth::Sample);
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let devices: Vec<WireJson> = self
            .bt
            .devices
            .iter()
            .map(|d| {
                WireJson::Obj(vec![
                    ("name", WireJson::s(&d.name)),
                    ("mac", WireJson::s(&d.mac)),
                    ("mac_b64", WireJson::s(b64(&d.mac))),
                    ("connected", WireJson::Bool(d.connected)),
                ])
            })
            .collect();

        out.set("bt_icon", self.icon());
        out.set(
            "bt",
            WireJson::Obj(vec![
                ("enabled", WireJson::Bool(self.bt.powered)),
                ("subtitle", WireJson::s(self.subtitle())),
                ("list_h", WireJson::Int(self.cfg.list_height(devices.len()))),
                ("devices", WireJson::Arr(devices)),
            ]),
        );
        out.set("bt_connecting", self.acting_on().unwrap_or(""));
    }

    fn poll_interval(&self) -> Option<u32> {
        Some(if self.menu_open {
            self.cfg.poll_open
        } else {
            self.cfg.poll_idle
        })
    }
}
