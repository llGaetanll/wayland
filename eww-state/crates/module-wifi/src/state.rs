//! The Wi-Fi state machine. No IO: everything here is arithmetic over state.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::millis::Millis;
use api::module::Module;
use api::util::b64;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionWifi;
use crate::config::ConfigWifi;
use crate::event::EventWifi;
use crate::model::NetSnapshot;
use crate::req::ReqWifi;

#[derive(Clone, Debug, Default)]
pub struct StateWifi {
    cfg: ConfigWifi,
    jobs: Jobs,
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

/// A job we are waiting on, and the tick we started waiting at.
#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

#[derive(Clone, Debug)]
struct Joining {
    id: JobId,
    ssid: String,
    since: Millis,
}

impl StateWifi {
    pub fn new(cfg: ConfigWifi) -> StateWifi {
        StateWifi {
            cfg,
            ..StateWifi::default()
        }
    }

    pub fn snapshot(&self) -> &NetSnapshot {
        &self.net
    }

    /// The SSID being joined, if one is.
    pub fn joining(&self) -> Option<&str> {
        self.connecting.as_ref().map(|j| j.ssid.as_str())
    }

    /// A radio toggle is out and has not been answered.
    pub fn settling(&self) -> bool {
        self.toggling.is_some()
    }

    pub fn icon(&self) -> String {
        if !self.net.enabled {
            return self.cfg.icon("network-wireless-disabled-symbolic");
        }
        match self.net.active_signal() {
            Some(s) => self.cfg.icon(signal_icon(s)),
            None if self.net.wired => self.cfg.icon("network-wired-symbolic"),
            None => self.cfg.icon("network-wireless-offline-symbolic"),
        }
    }

    fn subtitle(&self) -> &str {
        if !self.net.enabled {
            "Off"
        } else {
            self.net.connected.as_deref().unwrap_or("Not connected")
        }
    }

    /// Give up on a job that never reported back, so a switch cannot stay stuck
    /// mid-flip and a row cannot say "Connecting…" for the rest of the session.
    /// Asked on every tick; answered from state alone.
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
            .connecting
            .as_ref()
            .is_some_and(|j| self.now.since(j.since) >= timeout)
        {
            let Some(joining) = self.connecting.take() else {
                unreachable!("the branch above just matched on it")
            };
            self.say = Some(format!("Failed to connect to {}", joining.ssid));
            self.want_dismiss = true;
            self.want_sample = true;
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

impl Module for StateWifi {
    type Event = EventWifi;
    type Action = ActionWifi;
    type Req = ReqWifi;

    fn handle_event(&mut self, event: &EventWifi) {
        match event {
            EventWifi::Due => self.want_sample = true,
            EventWifi::Toggled => {
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
            EventWifi::Picked { ssid } => {
                // One join at a time; a second click is a mis-click.
                if self.connecting.is_none() && self.want_join.is_none() {
                    self.want_join = Some(ssid.clone());
                }
            }
            EventWifi::Observed { net } => {
                if self.toggling.is_none() && self.want_radio.is_none() {
                    self.net = net.clone();
                }
            }
            EventWifi::MenuOpened => {
                self.menu_open = true;
                self.want_scan = true;
                self.want_sample = true;
            }
            EventWifi::MenuClosed => self.menu_open = false,
            EventWifi::Tick { now } => {
                self.now = *now;
                self.expire();
            }
            EventWifi::Finished { id, ok, detail } => {
                let id = *id;
                if self.toggling.is_some_and(|t| t.id == id) {
                    self.toggling = None;
                    self.want_sample = true;
                } else if self.connecting.as_ref().is_some_and(|j| j.id == id) {
                    let Some(joining) = self.connecting.take() else {
                        unreachable!("the branch above just matched on it")
                    };
                    let ssid = joining.ssid;
                    self.say = match (ok, detail.as_deref()) {
                        (true, _) => Some(format!("Connected to {ssid}")),
                        // A cancelled password prompt is a deliberate no-op,
                        // not a failure worth a notification.
                        (false, Some("cancelled")) => None,
                        (false, _) => Some(format!("Failed to connect to {ssid}")),
                    };
                    self.want_dismiss = true;
                    self.want_sample = true;
                }
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionWifi>) -> Option<ReqWifi> {
        if let Some(on) = self.want_radio.take() {
            let id = self.jobs.take();
            actions.push(ActionWifi::SetEnabled { id, on });
            self.toggling = Some(Inflight {
                id,
                since: self.now,
            });
        }
        if std::mem::take(&mut self.want_scan) {
            actions.push(ActionWifi::Scan);
        }
        if let Some(ssid) = self.want_join.take() {
            if self.connecting.is_none() {
                let id = self.jobs.take();
                actions.push(ActionWifi::Connect {
                    id,
                    ssid: ssid.clone(),
                });
                self.connecting = Some(Joining {
                    id,
                    ssid,
                    since: self.now,
                });
            }
        }
        if let Some(body) = self.say.take() {
            actions.push(ActionWifi::Notify { body });
        }
        if std::mem::take(&mut self.want_sample) {
            actions.push(ActionWifi::Sample);
        }
        std::mem::take(&mut self.want_dismiss).then_some(ReqWifi::Dismiss)
    }

    fn vars(&self, out: &mut WireVars) {
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
                    ("icon", WireJson::s(self.cfg.icon(signal_icon(n.signal)))),
                ])
            })
            .collect();

        out.set("net_icon", self.icon());
        out.set(
            "wifi",
            WireJson::Obj(vec![
                ("enabled", WireJson::Bool(self.net.enabled)),
                ("subtitle", WireJson::s(self.subtitle())),
                (
                    "list_h",
                    WireJson::Int(self.cfg.list_height(networks.len())),
                ),
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

    fn poll_interval(&self) -> Option<u32> {
        Some(if self.menu_open {
            self.cfg.poll_open
        } else {
            self.cfg.poll_idle
        })
    }
}
