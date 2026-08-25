//! Wi-Fi: the bar icon and the network dropdown.

use crate::action::Action;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::millis::Millis;
use crate::model::NetSnapshot;
use crate::probe::JobId;
use crate::probe::Probe;
use crate::req::Req;
use crate::state::PollPlan;
use crate::state::State;
use crate::util::b64;
use crate::wire_json::WireJson;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateWifi {
    net: NetSnapshot,
    /// The network being joined, so its row can say "Connecting…".
    connecting: Option<Joining>,
    /// A radio on/off is in flight; samples taken while it settles would report
    /// the old state and flip the switch back under the user's finger.
    toggling: Option<Inflight>,
    menu_open: bool,
    now: Millis,

    /// What the batch asked for and has not been sent yet. Two clicks on the
    /// switch settle to one command, not two that undo each other.
    want_radio: Option<bool>,
    want_join: Option<String>,
    want_scan: bool,
    want_sample: bool,
    want_dismiss: bool,
    /// Told to the user once the batch is over, so a failure that is really a
    /// cancelled prompt can still be silent.
    say: Option<String>,
}

#[derive(Clone, Debug)]
struct Joining {
    id: JobId,
    ssid: String,
    since: Millis,
}

/// A job we are waiting on, and the tick we started waiting at.
#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

#[derive(Debug)]
pub enum EventWifi {
    ToggleRadio,
    Connect {
        ssid: String,
    },
    Observed {
        net: NetSnapshot,
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
        detail: Option<String>,
    },
}

impl StateWifi {
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

    /// Give up on a job that never reported back, so a switch cannot stay
    /// stuck mid-flip and a row cannot say "Connecting…" for the rest of the
    /// session. Asked on every tick; answered from state alone.
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
            .connecting
            .as_ref()
            .is_some_and(|j| now.since(j.since) >= cfg.job_timeout)
        {
            let Some(joining) = self.connecting.take() else {
                unreachable!("the branch above just matched on it")
            };
            self.say = Some(format!("Failed to connect to {}", joining.ssid));
            self.want_dismiss = true;
            self.want_sample = true;
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

impl State for StateWifi {
    type Event = EventWifi;

    fn handle_event(&mut self, cfg: &Config, event: EventWifi) {
        match event {
            EventWifi::ToggleRadio => {
                let on = !self.net.enabled;
                self.net.enabled = on;
                if !on {
                    // Nothing is reachable with the radio off; clearing now
                    // beats showing a list of ghosts until the next sample.
                    self.net.networks.clear();
                    self.net.connected = None;
                }
                self.want_radio = Some(on);
                self.want_scan = self.want_scan || on;
            }
            EventWifi::Connect { ssid } => {
                // One join at a time; a second click is a mis-click.
                if self.connecting.is_none() && self.want_join.is_none() {
                    self.want_join = Some(ssid);
                }
            }
            EventWifi::Observed { net } => {
                if self.toggling.is_none() && self.want_radio.is_none() {
                    self.net = net;
                }
            }
            EventWifi::MenuOpened => {
                self.menu_open = true;
                self.want_scan = true;
                self.want_sample = true;
            }
            EventWifi::MenuClosed => self.menu_open = false,
            EventWifi::Tick { now } => {
                self.now = now;
                self.expire(cfg);
            }
            EventWifi::Finished { id, ok, detail } => {
                if self.toggling.is_some_and(|t| t.id == id) {
                    self.toggling = None;
                    self.want_sample = true;
                } else if self.connecting.as_ref().is_some_and(|j| j.id == id) {
                    let Some(joining) = self.connecting.take() else {
                        unreachable!("the branch above just matched on it")
                    };
                    let ssid = joining.ssid;
                    self.say = match (ok, detail) {
                        (true, _) => Some(format!("Connected to {ssid}")),
                        // A cancelled password prompt is a deliberate no-op,
                        // not a failure worth a notification.
                        (false, Some(d)) if d == "cancelled" => None,
                        (false, _) => Some(format!("Failed to connect to {ssid}")),
                    };
                    self.want_dismiss = true;
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
        if let Some(on) = self.want_radio.take() {
            let id = jobs.take();
            actions.push(Action::shell(id, ActionShell::WifiSetEnabled { on }));
            self.toggling = Some(Inflight {
                id,
                since: self.now,
            });
        }
        if std::mem::take(&mut self.want_scan) {
            actions.push(Action::shell(jobs.take(), ActionShell::WifiScan));
        }
        if let Some(ssid) = self.want_join.take() {
            if self.connecting.is_none() {
                let id = jobs.take();
                actions.push(Action::shell(
                    id,
                    ActionShell::WifiConnect { ssid: ssid.clone() },
                ));
                self.connecting = Some(Joining {
                    id,
                    ssid,
                    since: self.now,
                });
            }
        }
        if let Some(body) = self.say.take() {
            actions.push(Action::notify(jobs.take(), "Wi-Fi", body));
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(Action::probe(jobs.take(), Probe::Net));
        }
        std::mem::take(&mut self.want_dismiss).then_some(Req::CloseMenu)
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
        let networks: Vec<WireJson> = self
            .net
            .networks
            .iter()
            .map(|n| {
                WireJson::Obj(vec![
                    ("ssid", WireJson::s(&n.ssid)),
                    ("ssid_b64", WireJson::s(b64(&n.ssid))),
                    ("signal", WireJson::Int(n.signal as i64)),
                    ("secure", WireJson::Bool(n.secure)),
                    ("active", WireJson::Bool(n.active)),
                    ("icon", WireJson::s(cfg.icon(signal_icon(n.signal)))),
                ])
            })
            .collect();

        out.set("net_icon", self.icon(cfg));
        out.set(
            "wifi",
            WireJson::Obj(vec![
                ("enabled", WireJson::Bool(self.net.enabled)),
                ("subtitle", WireJson::s(self.subtitle())),
                ("list_h", WireJson::Int(cfg.list_height(networks.len()))),
                ("networks", WireJson::Arr(networks)),
            ]),
        );
        out.set(
            "wifi_connecting",
            self.connecting
                .as_ref()
                .map(|j| j.ssid.as_str())
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
    use crate::action::Action;
    use crate::model::WifiNetwork;
    use crate::test_util::run;
    use crate::test_util::run_full;

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

    fn observed(enabled: bool, ssids: &[(&str, u8, bool)]) -> EventWifi {
        EventWifi::Observed {
            net: snap(enabled, ssids),
        }
    }

    fn notified(actions: &[Action]) -> bool {
        actions.iter().any(|a| {
            matches!(
                a,
                Action::ToShell {
                    action: ActionShell::Notify { .. },
                    ..
                }
            )
        })
    }

    #[test]
    fn toggle_ignores_samples_until_it_settles() {
        let mut w = StateWifi::default();
        run(&mut w, observed(true, &[("home", 90, true)]));
        let actions = run(&mut w, EventWifi::ToggleRadio);
        assert!(!w.net.enabled);
        assert!(w.net.networks.is_empty());

        // A sample taken before the radio actually went down.
        run(&mut w, observed(true, &[("home", 90, true)]));
        assert!(!w.net.enabled, "the in-flight toggle owns the state");

        run(
            &mut w,
            EventWifi::Finished {
                id: actions[0].id(),
                ok: true,
                detail: None,
            },
        );
        run(&mut w, observed(false, &[]));
        assert!(!w.net.enabled);
    }

    #[test]
    fn connect_shows_progress_then_closes_the_menu() {
        let mut w = StateWifi::default();
        run(&mut w, observed(true, &[("cafe", 60, false)]));
        let actions = run(
            &mut w,
            EventWifi::Connect {
                ssid: "cafe".into(),
            },
        );
        let id = actions[0].id();
        let mut vars = WireVars::default();
        w.vars(&Config::new("/i", "/c"), &mut vars);
        assert_eq!(vars.get("wifi_connecting"), Some("cafe"));

        // A second click while the first is running is dropped.
        assert!(run(
            &mut w,
            EventWifi::Connect {
                ssid: "other".into()
            }
        )
        .is_empty());

        let (actions, req) = run_full(
            &mut w,
            EventWifi::Finished {
                id,
                ok: true,
                detail: None,
            },
        );
        assert_eq!(req, Some(Req::CloseMenu));
        assert!(notified(&actions));
        assert!(w.connecting.is_none());
    }

    #[test]
    fn a_cancelled_password_prompt_is_silent() {
        let mut w = StateWifi::default();
        let actions = run(
            &mut w,
            EventWifi::Connect {
                ssid: "cafe".into(),
            },
        );
        let actions = run(
            &mut w,
            EventWifi::Finished {
                id: actions[0].id(),
                ok: false,
                detail: Some("cancelled".into()),
            },
        );
        assert!(!notified(&actions));
    }

    #[test]
    fn menu_polls_faster_than_the_bar_icon() {
        let cfg = Config::new("/i", "/c");
        let mut w = StateWifi::default();
        let mut plan = PollPlan::new();
        w.polls(&cfg, &mut plan);
        assert_eq!(plan[&Probe::Net], cfg.poll.net_idle);
        run(&mut w, EventWifi::MenuOpened);
        let mut plan = PollPlan::new();
        w.polls(&cfg, &mut plan);
        assert_eq!(plan[&Probe::Net], cfg.poll.net_open);
    }

    #[test]
    fn ethernet_shows_when_wifi_has_no_link() {
        let cfg = Config::new("/i", "/c");
        let mut w = StateWifi::default();
        let mut net = snap(true, &[("home", 90, false)]);
        net.wired = true;
        run(&mut w, EventWifi::Observed { net });
        assert!(w.icon(&cfg).ends_with("network-wired-symbolic.svg"));
    }
}
