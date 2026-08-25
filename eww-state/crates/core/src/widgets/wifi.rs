//! Wi-Fi: the bar icon and the network dropdown.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Req, Vars};
use crate::event::{JobId, Probe};
use crate::json::Json;
use crate::model::NetSnapshot;
use crate::util::b64;
use crate::widgets::{PollPlan, Widget};

#[derive(Clone, Debug, Default)]
pub struct Wifi {
    net: NetSnapshot,
    /// SSID being joined, so its row can say "Connecting…".
    connecting: Option<(JobId, String)>,
    /// A radio on/off is in flight; samples taken while it settles would report
    /// the old state and flip the switch back under the user's finger.
    toggling: Option<JobId>,
    menu_open: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    ToggleRadio,
    Connect {
        ssid: String,
    },
    Observed(NetSnapshot),
    MenuOpened,
    MenuClosed,
    Done {
        id: JobId,
        ok: bool,
        detail: Option<String>,
    },
}

impl Wifi {
    pub fn snapshot(&self) -> &NetSnapshot {
        &self.net
    }

    pub(crate) fn icon(&self, cfg: &Config) -> String {
        if !self.net.enabled {
            return cfg.icon("network-wireless-disabled-symbolic");
        }
        match self.net.active_signal() {
            Some(s) => cfg.icon(signal_icon(s)),
            None if self.net.wired => cfg.icon("network-wired-symbolic"),
            None => cfg.icon("network-wireless-offline-symbolic"),
        }
    }

    fn subtitle(&self) -> &str {
        if !self.net.enabled {
            "Off"
        } else {
            self.net.connected.as_deref().unwrap_or("Not connected")
        }
    }
}

pub(crate) fn signal_icon(signal: u8) -> &'static str {
    match signal {
        80..=255 => "nm-signal-100-symbolic",
        55..=79 => "nm-signal-75-symbolic",
        30..=54 => "nm-signal-50-symbolic",
        5..=29 => "nm-signal-25-symbolic",
        _ => "nm-signal-0-symbolic",
    }
}

impl Widget for Wifi {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::ToggleRadio => {
                let want = !self.net.enabled;
                self.net.enabled = want;
                if !want {
                    // Nothing is reachable with the radio off; clearing now
                    // beats showing a list of ghosts until the next sample.
                    self.net.networks.clear();
                    self.net.connected = None;
                }
                let id = cx.dispatch(ActionKind::WifiSetEnabled(want));
                self.toggling = Some(id);
                if want {
                    cx.dispatch(ActionKind::WifiScan);
                }
            }
            Ev::Connect { ssid } => {
                if self.connecting.is_some() {
                    return; // one join at a time; a second click is a mis-click
                }
                let id = cx.dispatch(ActionKind::WifiConnect { ssid: ssid.clone() });
                self.connecting = Some((id, ssid));
            }
            Ev::Observed(net) => {
                if self.toggling.is_none() {
                    self.net = net;
                }
            }
            Ev::MenuOpened => {
                self.menu_open = true;
                cx.dispatch(ActionKind::WifiScan);
                cx.dispatch(ActionKind::Probe(Probe::Net));
            }
            Ev::MenuClosed => self.menu_open = false,
            Ev::Done { id, ok, detail } => {
                if self.toggling == Some(id) {
                    self.toggling = None;
                    cx.dispatch(ActionKind::Probe(Probe::Net));
                } else if self.connecting.as_ref().is_some_and(|(j, _)| *j == id) {
                    let (_, ssid) = self.connecting.take().expect("checked above");
                    match (ok, detail) {
                        (true, _) => cx.notify("Wi-Fi", format!("Connected to {ssid}")),
                        // A cancelled password prompt is a deliberate no-op, not
                        // a failure worth a notification.
                        (false, Some(d)) if d == "cancelled" => {}
                        (false, _) => cx.notify("Wi-Fi", format!("Failed to connect to {ssid}")),
                    }
                    cx.request(Req::CloseMenu);
                    cx.dispatch(ActionKind::Probe(Probe::Net));
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut Vars) {
        let networks: Vec<Json> = self
            .net
            .networks
            .iter()
            .map(|n| {
                Json::Obj(vec![
                    ("ssid", Json::s(&n.ssid)),
                    ("ssid_b64", Json::s(b64(&n.ssid))),
                    ("signal", Json::Int(n.signal as i64)),
                    ("secure", Json::Bool(n.secure)),
                    ("active", Json::Bool(n.active)),
                    ("icon", Json::s(cfg.icon(signal_icon(n.signal)))),
                ])
            })
            .collect();

        out.set("net_icon", self.icon(cfg));
        out.set(
            "wifi",
            Json::Obj(vec![
                ("enabled", Json::Bool(self.net.enabled)),
                ("subtitle", Json::s(self.subtitle())),
                ("list_h", Json::Int(cfg.list_height(networks.len()))),
                ("networks", Json::Arr(networks)),
            ]),
        );
        out.set(
            "wifi_connecting",
            self.connecting
                .as_ref()
                .map(|(_, s)| s.as_str())
                .unwrap_or(""),
        );
    }

    fn polls(&self, cfg: &Config, out: &mut PollPlan) {
        out.insert(
            Probe::Net,
            if self.menu_open {
                cfg.poll.net_open
            } else {
                cfg.poll.net_idle
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WifiNetwork;
    use crate::test_util::{kinds, run};

    fn snap(enabled: bool, ssids: &[(&str, u8, bool)]) -> NetSnapshot {
        NetSnapshot {
            enabled,
            connected: ssids
                .iter()
                .find(|(_, _, a)| *a)
                .map(|(s, _, _)| s.to_string()),
            networks: ssids
                .iter()
                .map(|(s, sig, active)| WifiNetwork {
                    ssid: s.to_string(),
                    signal: *sig,
                    secure: true,
                    active: *active,
                })
                .collect(),
            wired: false,
        }
    }

    #[test]
    fn toggle_ignores_samples_until_it_settles() {
        let mut w = Wifi::default();
        run(&mut w, Ev::Observed(snap(true, &[("home", 90, true)])));
        let acts = run(&mut w, Ev::ToggleRadio);
        assert!(!w.net.enabled);
        assert!(w.net.networks.is_empty());

        // A sample taken before the radio actually went down.
        run(&mut w, Ev::Observed(snap(true, &[("home", 90, true)])));
        assert!(!w.net.enabled, "the in-flight toggle owns the state");

        let id = acts[0].id;
        run(
            &mut w,
            Ev::Done {
                id,
                ok: true,
                detail: None,
            },
        );
        run(&mut w, Ev::Observed(snap(false, &[])));
        assert!(!w.net.enabled);
    }

    #[test]
    fn connect_shows_progress_then_closes_the_menu() {
        let mut w = Wifi::default();
        run(&mut w, Ev::Observed(snap(true, &[("cafe", 60, false)])));
        let acts = run(
            &mut w,
            Ev::Connect {
                ssid: "cafe".into(),
            },
        );
        let id = acts[0].id;
        let mut vars = Vars::default();
        w.vars(&Config::new("/i", "/c"), &mut vars);
        assert_eq!(vars.get("wifi_connecting"), Some("cafe"));

        // A second click while the first is running is dropped.
        assert!(kinds(run(
            &mut w,
            Ev::Connect {
                ssid: "other".into()
            }
        ))
        .is_empty());

        let (acts, reqs) = crate::test_util::run_full(
            &mut w,
            Ev::Done {
                id,
                ok: true,
                detail: None,
            },
        );
        assert_eq!(reqs, vec![Req::CloseMenu]);
        assert!(kinds(acts)
            .iter()
            .any(|k| matches!(k, ActionKind::Notify { .. })));
        assert!(w.connecting.is_none());
    }

    #[test]
    fn a_cancelled_password_prompt_is_silent() {
        let mut w = Wifi::default();
        let acts = run(
            &mut w,
            Ev::Connect {
                ssid: "cafe".into(),
            },
        );
        let id = acts[0].id;
        let acts = run(
            &mut w,
            Ev::Done {
                id,
                ok: false,
                detail: Some("cancelled".into()),
            },
        );
        assert!(!kinds(acts)
            .iter()
            .any(|k| matches!(k, ActionKind::Notify { .. })));
    }

    #[test]
    fn menu_polls_faster_than_the_bar_icon() {
        let cfg = Config::new("/i", "/c");
        let mut w = Wifi::default();
        let mut plan = PollPlan::new();
        w.polls(&cfg, &mut plan);
        assert_eq!(plan[&Probe::Net], cfg.poll.net_idle);
        run(&mut w, Ev::MenuOpened);
        let mut plan = PollPlan::new();
        w.polls(&cfg, &mut plan);
        assert_eq!(plan[&Probe::Net], cfg.poll.net_open);
    }

    #[test]
    fn ethernet_shows_when_wifi_has_no_link() {
        let cfg = Config::new("/i", "/c");
        let mut w = Wifi::default();
        let mut net = snap(true, &[("home", 90, false)]);
        net.wired = true;
        run(&mut w, Ev::Observed(net));
        assert!(w.icon(&cfg).ends_with("network-wired-symbolic.svg"));
    }
}
