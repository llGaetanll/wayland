//! Bluetooth: the bar icon and the device dropdown.
//!
//! Shaped like [`crate::state_wifi`], with one deliberate difference: acting on
//! a device leaves the menu open, because pairing takes long enough that
//! watching the row update is the point.

use crate::action::Action;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::millis::Millis;
use crate::model::BtSnapshot;
use crate::probe::JobId;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::util::b64;
use crate::wire_json::WireJson;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateBluetooth {
    bt: BtSnapshot,
    /// MAC being connected or disconnected, plus whether we are tearing down.
    busy: Option<Busy>,
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

#[derive(Clone, Debug)]
struct Busy {
    id: JobId,
    mac: String,
    name: String,
    disconnecting: bool,
    since: Millis,
}

/// A job we are waiting on, and the tick we started waiting at.
#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

#[derive(Debug)]
pub enum EventBluetooth {
    TogglePower,
    /// Connect the device, or disconnect it if it is already connected.
    ToggleDevice {
        mac: String,
    },
    Observed {
        bt: BtSnapshot,
    },
    MenuOpened,
    MenuClosed,
    /// The clock moved; give up on anything that has waited too long.
    Tick {
        now: Millis,
    },
    Finished {
        id: JobId,
        ok: bool,
    },
}

impl StateBluetooth {
    pub fn snapshot(&self) -> &BtSnapshot {
        &self.bt
    }

    pub(crate) fn icon(&self, cfg: &Config) -> String {
        cfg.icon(if !self.bt.powered {
            "bluetooth-disabled-symbolic"
        } else if self.bt.devices.iter().any(|d| d.connected) {
            "bluetooth-paired-symbolic"
        } else {
            "bluetooth-active-symbolic"
        })
    }

    /// Give up on a job that never reported back. `bluetoothctl connect` on a
    /// device that has wandered out of range simply never returns, and without
    /// this the row would say "Connecting…" until the daemon restarts.
    fn expire(&mut self, cfg: &Config) {
        let now = self.now;
        if self
            .toggling
            .is_some_and(|t| now.since(t.since) >= cfg.job_timeout)
        {
            self.toggling = None;
            self.want_sample = true;
        }
        if self
            .busy
            .as_ref()
            .is_some_and(|b| now.since(b.since) >= cfg.job_timeout)
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

    /// Connect the device, or disconnect it if it is already connected. Reading
    /// the list here rather than at click time means a row that has since moved
    /// cannot send a command about a device that is no longer there.
    fn start_device(&mut self, jobs: &mut Jobs, actions: &mut Vec<Action>, mac: String) {
        if self.busy.is_some() {
            return;
        }
        let Some(dev) = self.bt.devices.iter().find(|d| d.mac == mac) else {
            return;
        };
        let (name, disconnecting) = (dev.name.clone(), dev.connected);
        let action = if disconnecting {
            ActionShell::BtDisconnect {
                mac: mac.clone(),
                name: name.clone(),
            }
        } else {
            ActionShell::BtConnect {
                mac: mac.clone(),
                name: name.clone(),
            }
        };
        let id = jobs.take();
        actions.push(Action::shell(id, action));
        self.busy = Some(Busy {
            id,
            mac,
            name,
            disconnecting,
            since: self.now,
        });
    }

    fn subtitle(&self) -> &str {
        if !self.bt.powered {
            "Off"
        } else {
            self.bt.connected_name().unwrap_or("On")
        }
    }
}

impl State for StateBluetooth {
    type Event = EventBluetooth;

    fn handle_event(&mut self, cfg: &Config, event: EventBluetooth) {
        match event {
            EventBluetooth::TogglePower => {
                let on = !self.bt.powered;
                self.bt.powered = on;
                if !on {
                    self.bt.devices.clear();
                }
                self.want_power = Some(on);
                self.want_scan = self.want_scan || on;
            }
            EventBluetooth::ToggleDevice { mac } => {
                if self.busy.is_none() && self.want_device.is_none() {
                    self.want_device = Some(mac);
                }
            }
            EventBluetooth::Observed { bt } => {
                if self.toggling.is_none() && self.want_power.is_none() {
                    self.bt = bt;
                }
            }
            EventBluetooth::MenuOpened => {
                self.menu_open = true;
                self.want_scan = self.want_scan || self.bt.powered;
                self.want_sample = true;
            }
            EventBluetooth::MenuClosed => self.menu_open = false,
            EventBluetooth::Tick { now } => {
                self.now = now;
                self.expire(cfg);
            }
            EventBluetooth::Finished { id, ok } => {
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

    fn emit_actions(
        &mut self,
        _cfg: &Config,
        jobs: &mut Jobs,
        actions: &mut Vec<Action>,
    ) -> Option<Req> {
        if let Some(on) = self.want_power.take() {
            let id = jobs.take();
            actions.push(Action::shell(id, ActionShell::BtSetPowered { on }));
            self.toggling = Some(Inflight {
                id,
                since: self.now,
            });
        }
        if std::mem::take(&mut self.want_scan) {
            actions.push(Action::shell(jobs.take(), ActionShell::BtScan));
        }
        if let Some(mac) = self.want_device.take() {
            self.start_device(jobs, actions, mac);
        }
        if let Some(body) = self.say.take() {
            actions.push(Action::notify(jobs.take(), "Bluetooth", body));
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(Action::probe(jobs.take(), Probe::Bt));
        }
        // A device click deliberately leaves the menu open: pairing takes long
        // enough that watching the row update is the point.
        None
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
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

        out.set("bt_icon", self.icon(cfg));
        out.set(
            "bt",
            WireJson::Obj(vec![
                ("enabled", WireJson::Bool(self.bt.powered)),
                ("subtitle", WireJson::s(self.subtitle())),
                ("list_h", WireJson::Int(cfg.list_height(devices.len()))),
                ("devices", WireJson::Arr(devices)),
            ]),
        );
        out.set(
            "bt_connecting",
            self.busy.as_ref().map(|b| b.mac.as_str()).unwrap_or(""),
        );
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(
            Probe::Bt,
            if self.menu_open {
                cfg.poll.bt_open
            } else {
                cfg.poll.bt_idle
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::model::BtDevice;
    use crate::test_util::run;

    fn observed(powered: bool, devices: &[(&str, &str, bool)]) -> EventBluetooth {
        EventBluetooth::Observed {
            bt: BtSnapshot {
                powered,
                devices: devices
                    .iter()
                    .map(|(mac, name, connected)| BtDevice {
                        mac: mac.to_string(),
                        name: name.to_string(),
                        connected: *connected,
                    })
                    .collect(),
            },
        }
    }

    fn click(mac: &str) -> EventBluetooth {
        EventBluetooth::ToggleDevice { mac: mac.into() }
    }

    #[test]
    fn a_click_disconnects_a_connected_device() {
        let mut b = StateBluetooth::default();
        run(&mut b, observed(true, &[("AA:BB", "Buds", true)]));
        let actions = run(&mut b, click("AA:BB"));
        assert!(matches!(
            actions.first(),
            Some(Action::ToShell {
                action: ActionShell::BtDisconnect { .. },
                ..
            })
        ));
    }

    #[test]
    fn a_click_connects_an_idle_device() {
        let mut b = StateBluetooth::default();
        run(&mut b, observed(true, &[("AA:BB", "Buds", false)]));
        let actions = run(&mut b, click("AA:BB"));
        assert!(matches!(
            actions.first(),
            Some(Action::ToShell {
                action: ActionShell::BtConnect { .. },
                ..
            })
        ));
    }

    #[test]
    fn unknown_device_is_ignored() {
        let mut b = StateBluetooth::default();
        run(&mut b, observed(true, &[]));
        assert!(run(&mut b, click("ZZ:ZZ")).is_empty());
    }

    #[test]
    fn one_operation_at_a_time() {
        let mut b = StateBluetooth::default();
        run(
            &mut b,
            observed(true, &[("AA:BB", "Buds", false), ("CC:DD", "Mouse", false)]),
        );
        run(&mut b, click("AA:BB"));
        assert!(run(&mut b, click("CC:DD")).is_empty());
    }

    #[test]
    fn notifies_with_the_name_it_started_with() {
        let mut b = StateBluetooth::default();
        run(&mut b, observed(true, &[("AA:BB", "Buds", false)]));
        let actions = run(&mut b, click("AA:BB"));
        let id = actions[0].id();
        // The device vanishes from the list mid-connect.
        run(&mut b, observed(true, &[]));
        let actions = run(&mut b, EventBluetooth::Finished { id, ok: true });
        let body = actions.into_iter().find_map(|a| match a {
            Action::ToShell {
                action: ActionShell::Notify { body, .. },
                ..
            } => Some(body),
            _ => None,
        });
        assert_eq!(body.as_deref(), Some("Connected Buds"));
    }
}
