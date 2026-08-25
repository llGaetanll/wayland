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

use crate::action::Action;
use crate::config::Config;
use crate::event::Event;
use crate::event_clock::EventClock;
use crate::event_eww::EventEww;
use crate::event_job::EventJob;
use crate::event_probe::EventProbe;
use crate::event_ui::EventUi;
use crate::jobs::Jobs;
use crate::millis::Millis;
use crate::model::MenuId;
use crate::probe::JobId;
use crate::probe::Probe;
use crate::req::Req;
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
use crate::state_id::StateId;
use crate::state_shot::EventShot;
use crate::state_shot::StateShot;
use crate::state_theme::EventTheme;
use crate::state_theme::StateTheme;
use crate::state_wifi::EventWifi;
use crate::state_wifi::StateWifi;
use crate::window::Window;
use crate::wire_vars::WireVars;

/// Granting a machine's request can make another machine speak, whose request
/// can in turn... Bounded so a mistake cannot spin forever.
const MAX_EMIT_ROUNDS: usize = 8;

pub struct StateBar {
    cfg: Config,
    /// The clock, as the last tick reported it. Nothing in here ever reads one.
    now: Millis,
    /// Which dropdown is open. The whole point of the type.
    menu: Option<MenuId>,
    /// The focused workspace holds a fullscreen window, so bar and dock hide.
    fullscreen: bool,
    jobs: Jobs,
    /// Which machine is waiting on each outstanding job.
    ///
    /// The bar fills this in structurally — it knows which machine it just
    /// asked to emit — but it only has to exist because every machine's actions
    /// share one enum and one id space, so a bare id cannot say where it came
    /// from. Give each machine its own action type and this goes away.
    owners: BTreeMap<JobId, StateId>,
    /// Probes the batch asked for. A set, so five things wanting fresh network
    /// data in one batch cost one sample.
    due: BTreeSet<Probe>,

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
    pub fn new(cfg: Config) -> Self {
        StateBar {
            cfg,
            now: Millis::default(),
            menu: None,
            fullscreen: false,
            jobs: Jobs::default(),
            owners: BTreeMap::new(),
            due: BTreeSet::new(),
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

    /// Apply a batch of events. This is the only way state changes, and it
    /// emits nothing: what the batch adds up to is not known until it is over.
    ///
    /// Deterministic, allocates no threads, touches no clock, blocks on nothing.
    pub fn handle_events(&mut self, events: &[Event]) {
        for event in events {
            self.handle_event(event);
        }
    }

    /// Say what has to happen now, given everything the batch left behind.
    pub fn emit_actions(&mut self, actions: &mut Vec<Action>) {
        for _ in 0..MAX_EMIT_ROUNDS {
            let reqs = self.emit_round(actions);
            if reqs.is_empty() {
                return;
            }
            // Granting a request routes events of its own, which may leave
            // another machine with something to say.
            for req in reqs {
                match req {
                    Req::CloseMenu => self.close_menu(),
                }
            }
        }
        debug_assert!(false, "requests never settled");
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

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::FromUi(e) => self.handle_event_ui(e),
            Event::FromClock(e) => self.handle_event_clock(e),
            Event::FromProbe(e) => self.handle_event_probe(e),
            Event::FromJob(e) => self.handle_event_job(e),
            Event::FromEww(e) => self.handle_event_eww(e),
        }
    }

    /// One pass over every machine, collecting what each asks of the bar.
    fn emit_round(&mut self, actions: &mut Vec<Action>) -> Vec<Req> {
        let mut reqs = Vec::new();
        for what in std::mem::take(&mut self.due) {
            actions.push(Action::probe(self.jobs.take(), what));
        }
        macro_rules! emit {
            ($machine:ident, $owner:expr) => {
                let start = actions.len();
                let req = self
                    .$machine
                    .emit_actions(&self.cfg, &mut self.jobs, actions);
                for action in &actions[start..] {
                    // A probe's result arrives as its own event and nobody waits
                    // on the job, so attributing it would only invite a loop.
                    if !matches!(action, Action::ToProbe { .. }) {
                        self.owners.insert(action.id(), $owner);
                    }
                }
                reqs.extend(req);
            };
        }
        emit!(audio, StateId::Audio);
        emit!(backlight, StateId::Backlight);
        emit!(wifi, StateId::Wifi);
        emit!(bluetooth, StateId::Bluetooth);
        emit!(battery, StateId::Battery);
        emit!(date, StateId::Date);
        emit!(theme, StateId::Theme);
        emit!(dock, StateId::Dock);
        emit!(shot, StateId::Shot);
        reqs
    }

    fn handle_event_ui(&mut self, event: &EventUi) {
        match event {
            EventUi::MenuToggled { menu } => {
                if self.menu == Some(*menu) {
                    self.close_menu();
                } else {
                    self.open_menu(*menu);
                }
            }
            EventUi::MenuDismissed => self.close_menu(),
            EventUi::VolumeSet { percent } => self
                .audio
                .handle_event(&self.cfg, EventAudio::Set { percent: *percent }),
            EventUi::BrightnessSet { percent } => self
                .backlight
                .handle_event(&self.cfg, EventBacklight::Set { percent: *percent }),
            EventUi::WifiToggled => self.wifi.handle_event(&self.cfg, EventWifi::ToggleRadio),
            EventUi::WifiPicked { ssid } => self
                .wifi
                .handle_event(&self.cfg, EventWifi::Connect { ssid: ssid.clone() }),
            EventUi::BtToggled => self
                .bluetooth
                .handle_event(&self.cfg, EventBluetooth::TogglePower),
            EventUi::BtPicked { mac } => self
                .bluetooth
                .handle_event(&self.cfg, EventBluetooth::ToggleDevice { mac: mac.clone() }),
            EventUi::DockLaunched { id } => self
                .dock
                .handle_event(&self.cfg, EventDock::Launch { id: id.clone() }),
            EventUi::DockDropped { moved, onto } => self.dock.handle_event(
                &self.cfg,
                EventDock::Swap {
                    moved: moved.clone(),
                    onto: onto.clone(),
                },
            ),
            EventUi::ShotRequested => self.shot.handle_event(&self.cfg, EventShot::Capture),
            EventUi::ShotActed { action } => self
                .shot
                .handle_event(&self.cfg, EventShot::Act { action: *action }),
        }
    }

    /// The clock moved. Nothing here decides what that means: the tick goes to
    /// every machine that has something to give up on, and each answers from
    /// its own state.
    fn handle_event_clock(&mut self, event: &EventClock) {
        let EventClock::Tick { now } = event;
        debug_assert!(*now >= self.now, "the clock went backwards");
        self.now = *now;
        self.wifi
            .handle_event(&self.cfg, EventWifi::Tick { now: *now });
        self.bluetooth
            .handle_event(&self.cfg, EventBluetooth::Tick { now: *now });
        self.shot
            .handle_event(&self.cfg, EventShot::Tick { now: *now });
    }

    fn handle_event_probe(&mut self, event: &EventProbe) {
        match event {
            EventProbe::Due { what } => {
                self.due.insert(*what);
            }
            EventProbe::Theme { theme } => self
                .theme
                .handle_event(&self.cfg, EventTheme::Observed { theme: *theme }),
            EventProbe::Date { text } => self
                .date
                .handle_event(&self.cfg, EventDate::Observed { text: text.clone() }),
            EventProbe::Battery { battery } => self
                .battery
                .handle_event(&self.cfg, EventBattery::Observed { battery: *battery }),
            EventProbe::Audio { audio } => self
                .audio
                .handle_event(&self.cfg, EventAudio::Observed { audio: *audio }),
            EventProbe::Backlight { percent } => self
                .backlight
                .handle_event(&self.cfg, EventBacklight::Observed { percent: *percent }),
            EventProbe::Net { net } => self
                .wifi
                .handle_event(&self.cfg, EventWifi::Observed { net: net.clone() }),
            EventProbe::Bt { bt } => self
                .bluetooth
                .handle_event(&self.cfg, EventBluetooth::Observed { bt: bt.clone() }),
            EventProbe::Dock { apps } => self
                .dock
                .handle_event(&self.cfg, EventDock::Observed { apps: apps.clone() }),
            EventProbe::Fullscreen { on } => self.fullscreen = *on,
        }
    }

    fn handle_event_job(&mut self, event: &EventJob) {
        match event {
            EventJob::ShotStaged { shot } => self
                .shot
                .handle_event(&self.cfg, EventShot::Staged { shot: shot.clone() }),
            EventJob::Finished { id, ok, detail } => {
                let (id, ok) = (*id, *ok);
                let Some(owner) = self.owners.remove(&id) else {
                    return; // a job from a previous life; nothing is waiting on it
                };
                match owner {
                    StateId::Audio => self.audio.handle_event(&self.cfg, EventAudio::Finished),
                    StateId::Backlight => self
                        .backlight
                        .handle_event(&self.cfg, EventBacklight::Finished),
                    StateId::Wifi => self.wifi.handle_event(
                        &self.cfg,
                        EventWifi::Finished {
                            id,
                            ok,
                            detail: detail.clone(),
                        },
                    ),
                    StateId::Bluetooth => self
                        .bluetooth
                        .handle_event(&self.cfg, EventBluetooth::Finished { id, ok }),
                    StateId::Battery => self
                        .battery
                        .handle_event(&self.cfg, EventBattery::Finished { id, ok }),
                    StateId::Shot => self
                        .shot
                        .handle_event(&self.cfg, EventShot::Finished { id }),
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

    fn open_menu(&mut self, m: MenuId) {
        if let Some(prev) = self.menu.replace(m) {
            self.tell_menu(prev, false);
        }
        self.tell_menu(m, true);
    }

    fn close_menu(&mut self) {
        if let Some(prev) = self.menu.take() {
            self.tell_menu(prev, false);
        }
    }

    /// Let the machine behind a dropdown know it came into or out of view, so
    /// it can refresh and choose its poll rate.
    fn tell_menu(&mut self, m: MenuId, opened: bool) {
        match m {
            MenuId::Wifi => {
                let event = if opened {
                    EventWifi::MenuOpened
                } else {
                    EventWifi::MenuClosed
                };
                self.wifi.handle_event(&self.cfg, event)
            }
            MenuId::Bt => {
                let event = if opened {
                    EventBluetooth::MenuOpened
                } else {
                    EventBluetooth::MenuClosed
                };
                self.bluetooth.handle_event(&self.cfg, event)
            }
            // The sliders have no open/closed state of their own; they just want
            // a fresh value the moment they appear.
            MenuId::Volume => {
                if opened {
                    self.due.insert(Probe::Audio);
                }
            }
            MenuId::Brightness => {
                if opened {
                    self.due.insert(Probe::Backlight);
                }
            }
        }
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
        StateBar::new(Config::new("/i", "/c"))
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

    /// Apply a batch and hand back what it asked for.
    fn turn(b: &mut StateBar, events: &[Event]) -> Vec<Action> {
        let mut actions = Vec::new();
        b.handle_events(events);
        b.emit_actions(&mut actions);
        actions
    }

    #[test]
    fn a_successful_join_dismisses_the_menu() {
        let mut b = bar();
        let actions = turn(
            &mut b,
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
        let actions = turn(
            &mut b,
            &[Event::FromProbe(EventProbe::Due { what: Probe::Audio })],
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
        let start = Millis::default();
        let timeout = b.config().job_timeout;
        eval!(&mut b,
            < Event::FromUi(EventUi::WifiToggled),
            > Action::ToShell { action: ActionShell::WifiSetEnabled { on: true }, .. },
            > Action::ToShell { action: ActionShell::WifiScan, .. },

            // The job is still out, so a tick inside the timeout changes nothing.
            < Event::FromClock(EventClock::Tick { now: start + Millis(timeout.0 / 2) }),

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
