//! The root state machine.
//!
//! The bar owns the sub-machines and the one thing none of them can own alone:
//! which dropdown is open. That lives here as a single `Option<MenuId>` — not a
//! boolean per menu — so "two menus open at once" is not a state that exists.
//!
//! Everything else it does is routing. An `Event` comes in, is translated into
//! at most one sub-machine's private event, and the actions that machine raised
//! come out. A machine that needs something of the bar (dismiss me) raises a
//! `Req` and the bar decides.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Instant;

use crate::action::Action;
use crate::config::Config;
use crate::ctx::Ctx;
use crate::ctx::Req;
use crate::ctx::StateId;
use crate::event::Event;
use crate::event_clock::EventClock;
use crate::event_eww::EventEww;
use crate::event_job::EventJob;
use crate::event_probe::EventProbe;
use crate::event_ui::EventUi;
use crate::model::MenuId;
use crate::probe::JobId;
use crate::probe::Probe;
use crate::state::PollPlan;
use crate::state::State;
use crate::state_audio::EventAudio;
use crate::state_audio::StateAudio;
use crate::state_backlight::EventBacklight;
use crate::state_backlight::StateBacklight;
use crate::state_battery::EventBattery;
use crate::state_battery::StateBattery;
use crate::state_bluetooth::EventBluetooth;
use crate::state_bluetooth::StateBluetooth;
use crate::state_date::EventDate;
use crate::state_date::StateDate;
use crate::state_dock::EventDock;
use crate::state_dock::StateDock;
use crate::state_shot::EventShot;
use crate::state_shot::StateShot;
use crate::state_theme::EventTheme;
use crate::state_theme::StateTheme;
use crate::state_wifi::EventWifi;
use crate::state_wifi::StateWifi;
use crate::window::Window;
use crate::wire_vars::WireVars;

/// A widget may ask the bar for something, and satisfying that may make another
/// widget speak. Bounded so a mistake cannot spin forever.
const MAX_REQ_ROUNDS: usize = 8;

pub struct StateBar {
    cfg: Config,
    /// The clock, as the last tick reported it. Nothing in here ever reads one.
    now: Instant,
    /// Which dropdown is open. The whole point of the type.
    menu: Option<MenuId>,
    /// The focused workspace holds a fullscreen window, so bar and dock hide.
    fullscreen: bool,
    next_job: u64,
    owners: BTreeMap<JobId, StateId>,

    audio: StateAudio,
    backlight: StateBacklight,
    wifi: StateWifi,
    bluetooth: StateBluetooth,
    battery: StateBattery,
    date: StateDate,
    theme: StateTheme,
    dock: StateDock,
    shot: StateShot,
}

impl StateBar {
    pub fn new(cfg: Config, now: Instant) -> Self {
        StateBar {
            cfg,
            now,
            menu: None,
            fullscreen: false,
            next_job: 0,
            owners: BTreeMap::new(),
            audio: StateAudio::default(),
            backlight: StateBacklight::default(),
            wifi: StateWifi::default(),
            bluetooth: StateBluetooth::default(),
            battery: StateBattery::default(),
            date: StateDate::default(),
            theme: StateTheme::default(),
            dock: StateDock::default(),
            shot: StateShot::default(),
        }
    }

    /// Apply a batch of events and collect the side effects they ask for.
    ///
    /// This is the only way state changes. It is deterministic, allocates no
    /// threads, touches no clock and blocks on nothing.
    pub fn handle_events(&mut self, events: &[Event], actions: &mut Vec<Action>) {
        for event in events {
            self.handle_event(event, actions);
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn menu(&self) -> Option<MenuId> {
        self.menu
    }

    pub fn audio(&self) -> &StateAudio {
        &self.audio
    }

    pub fn backlight(&self) -> &StateBacklight {
        &self.backlight
    }

    pub fn wifi(&self) -> &StateWifi {
        &self.wifi
    }

    pub fn bluetooth(&self) -> &StateBluetooth {
        &self.bluetooth
    }

    pub fn battery(&self) -> &StateBattery {
        &self.battery
    }

    pub fn theme(&self) -> &StateTheme {
        &self.theme
    }

    pub fn dock(&self) -> &StateDock {
        &self.dock
    }

    pub fn shot(&self) -> &StateShot {
        &self.shot
    }

    /// Every eww variable, as the machines currently describe themselves.
    ///
    /// Pure, and cheap enough to call after every batch: the runtime diffs it
    /// against what it last published, so eww only hears about real changes.
    pub fn vars(&self) -> WireVars {
        let mut v = WireVars::default();
        self.audio.vars(&self.cfg, &mut v);
        self.backlight.vars(&self.cfg, &mut v);
        self.wifi.vars(&self.cfg, &mut v);
        self.bluetooth.vars(&self.cfg, &mut v);
        self.battery.vars(&self.cfg, &mut v);
        self.date.vars(&self.cfg, &mut v);
        self.theme.vars(&self.cfg, &mut v);
        self.dock.vars(&self.cfg, &mut v);
        self.shot.vars(&self.cfg, &mut v);
        v.set("menu", self.menu.map(|m| m.as_str()).unwrap_or(""));
        v
    }

    /// The windows that should be on screen right now.
    ///
    /// Derived, never remembered: the runtime opens and closes to match, so the
    /// var and the actual surfaces cannot drift apart the way they did when
    /// each script opened its own.
    pub fn windows(&self) -> BTreeSet<Window> {
        let mut w = BTreeSet::new();
        if !self.fullscreen {
            w.insert(Window::Bar);
            w.insert(Window::Dock);
        }
        if let Some(m) = self.menu {
            w.insert(Window::Menu(m));
            w.insert(Window::MenuBackdrop);
        }
        if self.shot.is_open() {
            w.insert(Window::Shot);
            w.insert(Window::ShotBackdrop);
        }
        w
    }

    /// How often each probe should run, given what is on screen.
    pub fn polls(&self) -> PollPlan {
        let mut p = PollPlan::new();
        self.audio.polls(&self.cfg, &mut p);
        self.backlight.polls(&self.cfg, &mut p);
        self.wifi.polls(&self.cfg, &mut p);
        self.bluetooth.polls(&self.cfg, &mut p);
        self.battery.polls(&self.cfg, &mut p);
        self.date.polls(&self.cfg, &mut p);
        self.theme.polls(&self.cfg, &mut p);
        self.dock.polls(&self.cfg, &mut p);
        self.shot.polls(&self.cfg, &mut p);
        // Fullscreen is event-driven off the compositor, never sampled on a timer.
        p.insert(Probe::Fullscreen, 0);
        p
    }

    fn handle_event(&mut self, event: &Event, actions: &mut Vec<Action>) {
        let mut reqs = Vec::new();
        match event {
            Event::FromUi(e) => self.handle_event_ui(e, actions, &mut reqs),
            Event::FromClock(e) => self.handle_event_clock(e, actions, &mut reqs),
            Event::FromProbe(e) => self.handle_event_probe(e, actions, &mut reqs),
            Event::FromJob(e) => self.handle_event_job(e, actions, &mut reqs),
            Event::FromEww(e) => self.handle_event_eww(e),
        }
        self.settle(&mut reqs, actions);
    }

    fn handle_event_ui(&mut self, event: &EventUi, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        match event {
            EventUi::MenuToggled { menu } => {
                if self.menu == Some(*menu) {
                    self.close_menu(actions, reqs);
                } else {
                    self.open_menu(*menu, actions, reqs);
                }
            }
            EventUi::MenuDismissed => self.close_menu(actions, reqs),
            EventUi::VolumeSet { percent } => {
                let percent = *percent;
                self.with(StateId::Audio, actions, reqs, |b, cx| {
                    b.audio
                        .handle_event(&b.cfg, EventAudio::Set { percent }, cx)
                })
            }
            EventUi::BrightnessSet { percent } => {
                let percent = *percent;
                self.with(StateId::Backlight, actions, reqs, |b, cx| {
                    b.backlight
                        .handle_event(&b.cfg, EventBacklight::Set { percent }, cx)
                })
            }
            EventUi::WifiToggled => self.with(StateId::Wifi, actions, reqs, |b, cx| {
                b.wifi.handle_event(&b.cfg, EventWifi::ToggleRadio, cx)
            }),
            EventUi::WifiPicked { ssid } => {
                let ssid = ssid.clone();
                self.with(StateId::Wifi, actions, reqs, |b, cx| {
                    b.wifi.handle_event(&b.cfg, EventWifi::Connect { ssid }, cx)
                })
            }
            EventUi::BtToggled => self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
                b.bluetooth
                    .handle_event(&b.cfg, EventBluetooth::TogglePower, cx)
            }),
            EventUi::BtPicked { mac } => {
                let mac = mac.clone();
                self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
                    b.bluetooth
                        .handle_event(&b.cfg, EventBluetooth::ToggleDevice { mac }, cx)
                })
            }
            EventUi::DockLaunched { id } => {
                let id = id.clone();
                self.with(StateId::Dock, actions, reqs, |b, cx| {
                    b.dock.handle_event(&b.cfg, EventDock::Launch { id }, cx)
                })
            }
            EventUi::DockDropped { moved, onto } => {
                let (moved, onto) = (moved.clone(), onto.clone());
                self.with(StateId::Dock, actions, reqs, |b, cx| {
                    b.dock
                        .handle_event(&b.cfg, EventDock::Swap { moved, onto }, cx)
                })
            }
            EventUi::ShotRequested => self.with(StateId::Shot, actions, reqs, |b, cx| {
                b.shot.handle_event(&b.cfg, EventShot::Capture, cx)
            }),
            EventUi::ShotActed { action } => {
                let action = *action;
                self.with(StateId::Shot, actions, reqs, |b, cx| {
                    b.shot.handle_event(&b.cfg, EventShot::Act { action }, cx)
                })
            }
        }
    }

    /// The clock moved. Nothing here decides what that means: the tick goes to
    /// every machine that has something to give up on, and each answers from
    /// its own state.
    fn handle_event_clock(
        &mut self,
        event: &EventClock,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
    ) {
        let EventClock::Tick { now } = event;
        debug_assert!(*now >= self.now, "the clock went backwards");
        self.now = *now;
        self.with(StateId::Wifi, actions, reqs, |b, cx| {
            b.wifi.handle_event(&b.cfg, EventWifi::Tick, cx)
        });
        self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
            b.bluetooth.handle_event(&b.cfg, EventBluetooth::Tick, cx)
        });
        self.with(StateId::Shot, actions, reqs, |b, cx| {
            b.shot.handle_event(&b.cfg, EventShot::Tick, cx)
        });
    }

    fn handle_event_probe(
        &mut self,
        event: &EventProbe,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
    ) {
        match event {
            EventProbe::Due { what } => {
                let what = *what;
                self.with(StateId::Bar, actions, reqs, |_, cx| {
                    cx.probe(what);
                })
            }
            EventProbe::Theme { theme } => {
                let theme = *theme;
                self.with(StateId::Theme, actions, reqs, |b, cx| {
                    b.theme
                        .handle_event(&b.cfg, EventTheme::Observed { theme }, cx)
                })
            }
            EventProbe::Date { text } => {
                let text = text.clone();
                self.with(StateId::Date, actions, reqs, |b, cx| {
                    b.date
                        .handle_event(&b.cfg, EventDate::Observed { text }, cx)
                })
            }
            EventProbe::Battery { battery } => {
                let battery = *battery;
                self.with(StateId::Battery, actions, reqs, |b, cx| {
                    b.battery
                        .handle_event(&b.cfg, EventBattery::Observed { battery }, cx)
                })
            }
            EventProbe::Audio { audio } => {
                let audio = *audio;
                self.with(StateId::Audio, actions, reqs, |b, cx| {
                    b.audio
                        .handle_event(&b.cfg, EventAudio::Observed { audio }, cx)
                })
            }
            EventProbe::Backlight { percent } => {
                let percent = *percent;
                self.with(StateId::Backlight, actions, reqs, |b, cx| {
                    b.backlight
                        .handle_event(&b.cfg, EventBacklight::Observed { percent }, cx)
                })
            }
            EventProbe::Net { net } => {
                let net = net.clone();
                self.with(StateId::Wifi, actions, reqs, |b, cx| {
                    b.wifi.handle_event(&b.cfg, EventWifi::Observed { net }, cx)
                })
            }
            EventProbe::Bt { bt } => {
                let bt = bt.clone();
                self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
                    b.bluetooth
                        .handle_event(&b.cfg, EventBluetooth::Observed { bt }, cx)
                })
            }
            EventProbe::Dock { apps } => {
                let apps = apps.clone();
                self.with(StateId::Dock, actions, reqs, |b, cx| {
                    b.dock
                        .handle_event(&b.cfg, EventDock::Observed { apps }, cx)
                })
            }
            EventProbe::Fullscreen { on } => self.fullscreen = *on,
        }
    }

    fn handle_event_job(
        &mut self,
        event: &EventJob,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
    ) {
        match event {
            EventJob::ShotStaged { shot } => {
                let shot = shot.clone();
                self.with(StateId::Shot, actions, reqs, |b, cx| {
                    b.shot.handle_event(&b.cfg, EventShot::Staged { shot }, cx)
                })
            }
            EventJob::Finished { id, ok, detail } => {
                let (id, ok) = (*id, *ok);
                let Some(owner) = self.owners.remove(&id) else {
                    return; // a job from a previous life; nothing is waiting on it
                };
                match owner {
                    StateId::Audio => self.with(StateId::Audio, actions, reqs, |b, cx| {
                        b.audio.handle_event(&b.cfg, EventAudio::Finished, cx)
                    }),
                    StateId::Backlight => self.with(StateId::Backlight, actions, reqs, |b, cx| {
                        b.backlight
                            .handle_event(&b.cfg, EventBacklight::Finished, cx)
                    }),
                    StateId::Wifi => {
                        let detail = detail.clone();
                        self.with(StateId::Wifi, actions, reqs, |b, cx| {
                            b.wifi
                                .handle_event(&b.cfg, EventWifi::Finished { id, ok, detail }, cx)
                        })
                    }
                    StateId::Bluetooth => self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
                        b.bluetooth
                            .handle_event(&b.cfg, EventBluetooth::Finished { id, ok }, cx)
                    }),
                    StateId::Battery => self.with(StateId::Battery, actions, reqs, |b, cx| {
                        b.battery
                            .handle_event(&b.cfg, EventBattery::Finished { id, ok }, cx)
                    }),
                    StateId::Shot => self.with(StateId::Shot, actions, reqs, |b, cx| {
                        b.shot.handle_event(&b.cfg, EventShot::Finished { id }, cx)
                    }),
                    StateId::Date | StateId::Theme | StateId::Dock | StateId::Bar => {}
                }
            }
        }
    }

    /// Nothing to change: the runtime republishes vars and windows from the
    /// state we already hold, which is exactly what recovery needs.
    fn handle_event_eww(&mut self, event: &EventEww) {
        match event {
            EventEww::Restarted => {}
        }
    }

    /// Grant what the sub-machines asked for, and whatever that in turn raises.
    fn settle(&mut self, reqs: &mut Vec<Req>, actions: &mut Vec<Action>) {
        for _ in 0..MAX_REQ_ROUNDS {
            if reqs.is_empty() {
                return;
            }
            for req in std::mem::take(reqs) {
                match req {
                    Req::CloseMenu => self.close_menu(actions, reqs),
                }
            }
        }
        debug_assert!(reqs.is_empty(), "requests never settled: {reqs:?}");
    }

    fn open_menu(&mut self, m: MenuId, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        if let Some(prev) = self.menu.replace(m) {
            self.tell_menu(prev, false, actions, reqs);
        }
        self.tell_menu(m, true, actions, reqs);
    }

    fn close_menu(&mut self, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        if let Some(prev) = self.menu.take() {
            self.tell_menu(prev, false, actions, reqs);
        }
    }

    /// Let the machine behind a dropdown know it came into or out of view, so
    /// it can refresh and choose its poll rate.
    fn tell_menu(
        &mut self,
        m: MenuId,
        opened: bool,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
    ) {
        match m {
            MenuId::Wifi => self.with(StateId::Wifi, actions, reqs, |b, cx| {
                let event = if opened {
                    EventWifi::MenuOpened
                } else {
                    EventWifi::MenuClosed
                };
                b.wifi.handle_event(&b.cfg, event, cx)
            }),
            MenuId::Bt => self.with(StateId::Bluetooth, actions, reqs, |b, cx| {
                let event = if opened {
                    EventBluetooth::MenuOpened
                } else {
                    EventBluetooth::MenuClosed
                };
                b.bluetooth.handle_event(&b.cfg, event, cx)
            }),
            // The sliders have no open/closed state of their own; they just want
            // a fresh value the moment they appear.
            MenuId::Volume => {
                if opened {
                    self.with(StateId::Audio, actions, reqs, |_, cx| {
                        cx.probe(Probe::Audio);
                    })
                }
            }
            MenuId::Brightness => {
                if opened {
                    self.with(StateId::Backlight, actions, reqs, |_, cx| {
                        cx.probe(Probe::Backlight);
                    })
                }
            }
        }
    }

    /// Run `f` with a `Ctx` that attributes anything it dispatches to `owner`.
    fn with(
        &mut self,
        owner: StateId,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
        f: impl FnOnce(&mut StateBar, &mut Ctx),
    ) {
        // Split the borrow: the closure gets the sub-machines, the Ctx gets the
        // bookkeeping. Taking them out and putting them back is the price of
        // keeping their code free of any reference to the bar.
        let now = self.now;
        let mut next_job = self.next_job;
        let mut owners = std::mem::take(&mut self.owners);
        {
            let mut cx = Ctx::new(owner, now, &mut next_job, actions, &mut owners, reqs);
            f(self, &mut cx);
        }
        self.next_job = next_job;
        self.owners = owners;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_probe::ActionProbe;
    use crate::action_shell::ActionShell;
    use crate::model::NetSnapshot;
    use crate::model::Shot;
    use crate::model::ShotAction;
    use crate::model::WifiNetwork;
    use eval::eval;

    fn bar() -> StateBar {
        StateBar::new(Config::new("/i", "/c"), Instant::now())
    }

    fn names(b: &StateBar) -> Vec<String> {
        b.windows().into_iter().map(|w| w.name()).collect()
    }

    fn toggle(menu: MenuId) -> Event {
        Event::FromUi(EventUi::MenuToggled { menu })
    }

    #[test]
    fn only_one_menu_can_be_open() {
        let mut b = bar();
        eval!(&mut b,
            < toggle(MenuId::Wifi),
            > Action::ToShell { action: ActionShell::WifiScan, .. },
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Net }, .. },

            < toggle(MenuId::Bt),
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Bt }, .. },
        );
        assert_eq!(b.menu(), Some(MenuId::Bt));
        let open = names(&b);
        assert!(open.contains(&"bt-menu".to_string()));
        assert!(!open.contains(&"wifi-menu".to_string()));
        assert!(open.contains(&"menu-backdrop".to_string()));
    }

    #[test]
    fn toggling_the_open_menu_closes_it() {
        let mut b = bar();
        eval!(&mut b,
            < toggle(MenuId::Volume),
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Audio }, .. },

            < toggle(MenuId::Volume),
        );
        assert_eq!(b.menu(), None);
        assert!(!names(&b).iter().any(|n| n.ends_with("-menu")));
        assert!(!names(&b).contains(&"menu-backdrop".to_string()));
    }

    #[test]
    fn the_screenshot_preview_is_independent_of_bar_menus() {
        let mut b = bar();
        eval!(&mut b,
            < toggle(MenuId::Wifi),
            > Action::ToShell { action: ActionShell::WifiScan, .. },
            > Action::ToProbe { .. },

            < Event::FromJob(EventJob::ShotStaged {
                shot: Shot { path: "/a.png".into(), width: 100, height: 50 },
            }),
        );
        assert_eq!(
            b.menu(),
            Some(MenuId::Wifi),
            "a capture leaves the menu alone"
        );
        assert!(names(&b).contains(&"shot-menu".to_string()));

        eval!(&mut b,
            < Event::FromUi(EventUi::ShotActed { action: ShotAction::Dismiss }),
        );
        assert_eq!(b.menu(), Some(MenuId::Wifi));
        assert!(!names(&b).contains(&"shot-menu".to_string()));
    }

    #[test]
    fn a_successful_join_dismisses_the_menu() {
        let mut b = bar();
        let mut actions = Vec::new();
        b.handle_events(
            &[
                Event::FromProbe(EventProbe::Net {
                    net: NetSnapshot {
                        enabled: true,
                        connected: None,
                        networks: vec![WifiNetwork {
                            ssid: "cafe".into(),
                            signal: 70,
                            secure: true,
                            active: false,
                        }],
                        wired: false,
                    },
                }),
                toggle(MenuId::Wifi),
                Event::FromUi(EventUi::WifiPicked {
                    ssid: "cafe".into(),
                }),
            ],
            &mut actions,
        );
        let id = actions
            .iter()
            .find(|a| {
                matches!(
                    a,
                    Action::ToShell {
                        action: ActionShell::WifiConnect { .. },
                        ..
                    }
                )
            })
            .expect("dispatched a join")
            .id();

        eval!(&mut b,
            < Event::FromJob(EventJob::Finished { id, ok: true, detail: None }),
            > Action::ToShell { action: ActionShell::Notify { .. }, .. },
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Net }, .. },
        );
        assert_eq!(b.menu(), None, "the machine asked the bar to dismiss it");
    }

    #[test]
    fn fullscreen_hides_bar_and_dock_together() {
        let mut b = bar();
        assert_eq!(names(&b), ["bar", "dock"]);
        eval!(&mut b,
            < Event::FromProbe(EventProbe::Fullscreen { on: true }),
        );
        assert!(names(&b).is_empty());

        // A menu is still reachable from a keybind while the bar is hidden.
        eval!(&mut b,
            < toggle(MenuId::Volume),
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Audio }, .. },

            < Event::FromProbe(EventProbe::Fullscreen { on: false }),
        );
        assert_eq!(names(&b), ["bar", "dock", "volume-menu", "menu-backdrop"]);
    }

    #[test]
    fn a_probe_result_does_not_ask_for_another_probe() {
        let mut b = bar();
        let mut actions = Vec::new();
        b.handle_events(
            &[Event::FromProbe(EventProbe::Due { what: Probe::Audio })],
            &mut actions,
        );
        let id = actions[0].id();

        // A probe finishing must not start another one.
        eval!(&mut b,
            < Event::FromJob(EventJob::Finished { id, ok: true, detail: None }),
        );
    }

    #[test]
    fn a_job_from_a_previous_life_is_ignored() {
        let mut b = bar();
        let before = b.vars();
        eval!(&mut b,
            < Event::FromJob(EventJob::Finished { id: JobId(999), ok: true, detail: None }),
        );
        assert_eq!(b.vars(), before);
    }

    #[test]
    fn a_toggle_that_never_reports_back_lets_go_on_its_own() {
        let mut b = bar();
        let start = Instant::now();
        let timeout = b.config().job_timeout;
        eval!(&mut b,
            < Event::FromUi(EventUi::WifiToggled),
            > Action::ToShell { action: ActionShell::WifiSetEnabled { on: true }, .. },
            > Action::ToShell { action: ActionShell::WifiScan, .. },

            // The job is still out, so a tick inside the timeout changes nothing.
            < Event::FromClock(EventClock::Tick { now: start + timeout / 2 }),

            < Event::FromClock(EventClock::Tick { now: start + timeout }),
            > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Net }, .. },
        );
        assert!(
            b.wifi().snapshot().enabled,
            "giving up on the job must not undo what the user asked for"
        );
    }

    #[test]
    fn vars_cover_every_widget() {
        let v = bar().vars();
        for name in [
            "theme",
            "clock",
            "menu",
            "vol_pct",
            "vol_icon",
            "bri_pct",
            "bri_icon",
            "net_icon",
            "wifi",
            "wifi_connecting",
            "bt_icon",
            "bt",
            "bt_connecting",
            "batt_icon",
            "dock_apps",
            "shot_path",
            "shot_w",
            "shot_h",
        ] {
            assert!(v.get(name).is_some(), "missing var {name}");
        }
    }
}
