//! Bluetooth: the bar icon and the device dropdown.
//!
//! Shaped like [`crate::widgets::wifi`], with one deliberate difference: acting
//! on a device leaves the menu open, because pairing takes long enough that
//! watching the row update is the point.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::{JobId, Probe};
use crate::json::Json;
use crate::model::BtSnapshot;
use crate::util::b64;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Bluetooth {
    bt: BtSnapshot,
    /// MAC being connected or disconnected, plus whether we are tearing down.
    busy: Option<Busy>,
    toggling: Option<JobId>,
    menu_open: bool,
}

#[derive(Clone, Debug)]
struct Busy {
    id: JobId,
    mac: String,
    name: String,
    disconnecting: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    TogglePower,
    /// Connect the device, or disconnect it if it is already connected.
    ToggleDevice {
        mac: String,
    },
    Observed(BtSnapshot),
    MenuOpened,
    MenuClosed,
    Done {
        id: JobId,
        ok: bool,
    },
}

impl Bluetooth {
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

    fn subtitle(&self) -> &str {
        if !self.bt.powered {
            "Off"
        } else {
            self.bt.connected_name().unwrap_or("On")
        }
    }
}

impl Widget for Bluetooth {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::TogglePower => {
                let want = !self.bt.powered;
                self.bt.powered = want;
                if !want {
                    self.bt.devices.clear();
                }
                let id = cx.dispatch(ActionKind::BtSetPowered(want));
                self.toggling = Some(id);
                if want {
                    cx.dispatch(ActionKind::BtScan);
                }
            }
            Ev::ToggleDevice { mac } => {
                if self.busy.is_some() {
                    return;
                }
                let Some(dev) = self.bt.devices.iter().find(|d| d.mac == mac) else {
                    return; // the list moved under the click; ignore it
                };
                let (name, disconnecting) = (dev.name.clone(), dev.connected);
                let kind = if disconnecting {
                    ActionKind::BtDisconnect {
                        mac: mac.clone(),
                        name: name.clone(),
                    }
                } else {
                    ActionKind::BtConnect {
                        mac: mac.clone(),
                        name: name.clone(),
                    }
                };
                let id = cx.dispatch(kind);
                self.busy = Some(Busy {
                    id,
                    mac,
                    name,
                    disconnecting,
                });
            }
            Ev::Observed(bt) => {
                if self.toggling.is_none() {
                    self.bt = bt;
                }
            }
            Ev::MenuOpened => {
                self.menu_open = true;
                if self.bt.powered {
                    cx.dispatch(ActionKind::BtScan);
                }
                cx.dispatch(ActionKind::Probe(Probe::Bt));
            }
            Ev::MenuClosed => self.menu_open = false,
            Ev::Done { id, ok } => {
                if self.toggling == Some(id) {
                    self.toggling = None;
                    cx.dispatch(ActionKind::Probe(Probe::Bt));
                } else if self.busy.as_ref().is_some_and(|b| b.id == id) {
                    let b = self.busy.take().expect("checked above");
                    let msg = match (b.disconnecting, ok) {
                        (true, true) => format!("Disconnected {}", b.name),
                        (true, false) => format!("Failed to disconnect {}", b.name),
                        (false, true) => format!("Connected {}", b.name),
                        (false, false) => format!("Failed to connect {}", b.name),
                    };
                    cx.notify("Bluetooth", msg);
                    cx.dispatch(ActionKind::Probe(Probe::Bt));
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut Vars) {
        let devices: Vec<Json> = self
            .bt
            .devices
            .iter()
            .map(|d| {
                Json::Obj(vec![
                    ("name", Json::s(&d.name)),
                    ("mac", Json::s(&d.mac)),
                    ("mac_b64", Json::s(b64(&d.mac))),
                    ("connected", Json::Bool(d.connected)),
                ])
            })
            .collect();

        out.set("bt_icon", self.icon(cfg));
        out.set(
            "bt",
            Json::Obj(vec![
                ("enabled", Json::Bool(self.bt.powered)),
                ("subtitle", Json::s(self.subtitle())),
                ("list_h", Json::Int(cfg.list_height(devices.len()))),
                ("devices", Json::Arr(devices)),
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
    use crate::model::BtDevice;
    use crate::test_util::{kinds, run};

    fn snap(powered: bool, devices: &[(&str, &str, bool)]) -> BtSnapshot {
        BtSnapshot {
            powered,
            devices: devices
                .iter()
                .map(|(mac, name, connected)| BtDevice {
                    mac: mac.to_string(),
                    name: name.to_string(),
                    connected: *connected,
                })
                .collect(),
        }
    }

    #[test]
    fn a_click_disconnects_a_connected_device() {
        let mut b = Bluetooth::default();
        run(&mut b, Ev::Observed(snap(true, &[("AA:BB", "Buds", true)])));
        let acts = run(
            &mut b,
            Ev::ToggleDevice {
                mac: "AA:BB".into(),
            },
        );
        assert!(matches!(
            kinds(acts).first(),
            Some(ActionKind::BtDisconnect { .. })
        ));
    }

    #[test]
    fn a_click_connects_an_idle_device() {
        let mut b = Bluetooth::default();
        run(
            &mut b,
            Ev::Observed(snap(true, &[("AA:BB", "Buds", false)])),
        );
        let acts = run(
            &mut b,
            Ev::ToggleDevice {
                mac: "AA:BB".into(),
            },
        );
        assert!(matches!(
            kinds(acts).first(),
            Some(ActionKind::BtConnect { .. })
        ));
    }

    #[test]
    fn unknown_device_is_ignored() {
        let mut b = Bluetooth::default();
        run(&mut b, Ev::Observed(snap(true, &[])));
        assert!(kinds(run(
            &mut b,
            Ev::ToggleDevice {
                mac: "ZZ:ZZ".into()
            }
        ))
        .is_empty());
    }

    #[test]
    fn one_operation_at_a_time() {
        let mut b = Bluetooth::default();
        run(
            &mut b,
            Ev::Observed(snap(
                true,
                &[("AA:BB", "Buds", false), ("CC:DD", "Mouse", false)],
            )),
        );
        run(
            &mut b,
            Ev::ToggleDevice {
                mac: "AA:BB".into(),
            },
        );
        assert!(kinds(run(
            &mut b,
            Ev::ToggleDevice {
                mac: "CC:DD".into()
            }
        ))
        .is_empty());
    }

    #[test]
    fn notifies_with_the_name_it_started_with() {
        let mut b = Bluetooth::default();
        run(
            &mut b,
            Ev::Observed(snap(true, &[("AA:BB", "Buds", false)])),
        );
        let acts = run(
            &mut b,
            Ev::ToggleDevice {
                mac: "AA:BB".into(),
            },
        );
        let id = acts[0].id;
        // The device vanishes from the list mid-connect.
        run(&mut b, Ev::Observed(snap(true, &[])));
        let acts = run(&mut b, Ev::Done { id, ok: true });
        let notified = kinds(acts).into_iter().find_map(|k| match k {
            ActionKind::Notify { body, .. } => Some(body),
            _ => None,
        });
        assert_eq!(notified.as_deref(), Some("Connected Buds"));
    }
}
