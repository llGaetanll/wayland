//! The root state machine.
//!
//! The bar owns the modules and the two facts none of them can own alone: which
//! dropdown is open, and whether a fullscreen Space is focused. The first lives
//! here as a single `Option<MenuId>` — not a boolean per menu — so "two menus
//! open at once" is not a state that exists.
//!
//! Everything else it does is routing. An event goes to exactly one module, and
//! that module's actions come back out re-labelled. A module that needs
//! something of the bar raises a request and the bar decides — which is the
//! only direction any of them can speak in.

use std::collections::BTreeSet;

use api::millis::Millis;
use api::module::Module;
use api::wire_vars::WireVars;
use module_audio::EventAudio;
use module_audio::StateAudio;
use module_backlight::EventBacklight;
use module_backlight::StateBacklight;
use module_battery::StateBattery;
use module_bluetooth::EventBluetooth;
use module_bluetooth::StateBluetooth;
use module_date::StateDate;
use module_dock::StateDock;
use module_shot::EventShot;
use module_shot::StateShot;
use module_theme::StateTheme;
use module_wifi::EventWifi;
use module_wifi::ReqWifi;
use module_wifi::StateWifi;

use crate::action::Action;
use crate::config::ConfigBar;
use crate::event::Event;
use crate::event_bar::EventBar;
use crate::event_clock::EventClock;
use crate::menu::MenuId;
use crate::module_id::ModuleId;
use crate::module_id::PollPlan;
use crate::window::Window;

/// Granting a module's request can make another module speak, whose request can
/// in turn... Bounded so a mistake cannot spin forever.
const MAX_EMIT_ROUNDS: usize = 8;

#[derive(Default)]
pub struct StateBar {
    /// Which dropdown is open. The whole point of the type.
    menu: Option<MenuId>,
    /// The focused workspace holds a fullscreen window, so bar and dock hide.
    fullscreen: bool,
    /// A fullscreen re-check the batch asked for, collapsed: a burst of
    /// compositor events is one question.
    fullscreen_due: bool,
    now: Millis,

    audio: StateAudio,
    backlight: StateBacklight,
    battery: StateBattery,
    bluetooth: StateBluetooth,
    date: StateDate,
    dock: StateDock,
    shot: StateShot,
    theme: StateTheme,
    wifi: StateWifi,
}

impl StateBar {
    pub fn new(cfg: ConfigBar) -> StateBar {
        StateBar {
            audio: StateAudio::new(cfg.audio),
            backlight: StateBacklight::new(cfg.backlight),
            battery: StateBattery::new(cfg.battery),
            bluetooth: StateBluetooth::new(cfg.bluetooth),
            date: StateDate::new(cfg.date),
            dock: StateDock::new(cfg.dock),
            shot: StateShot::new(cfg.shot),
            theme: StateTheme::new(cfg.theme),
            wifi: StateWifi::new(cfg.wifi),
            ..StateBar::default()
        }
    }

    /// Apply a batch of events. This is the only way state changes, and it
    /// emits nothing: what the batch adds up to is not known until it is over.
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
            // another module with something to say.
            for req in reqs {
                match req {
                    ReqWifi::Dismiss => self.close_menu(),
                }
            }
        }
        debug_assert!(false, "requests never settled");
    }

    pub fn menu(&self) -> Option<MenuId> {
        self.menu
    }

    pub fn wifi(&self) -> &StateWifi {
        &self.wifi
    }

    pub fn bluetooth(&self) -> &StateBluetooth {
        &self.bluetooth
    }

    pub fn dock(&self) -> &StateDock {
        &self.dock
    }

    pub fn date(&self) -> &StateDate {
        &self.date
    }

    /// Every eww variable, as the modules currently describe themselves.
    ///
    /// Pure, and cheap enough to call after every batch: the runtime diffs it
    /// against what it last published, so eww only hears about real changes.
    pub fn vars(&self) -> WireVars {
        let mut v = WireVars::default();
        self.audio.vars(&mut v);
        self.backlight.vars(&mut v);
        self.battery.vars(&mut v);
        self.bluetooth.vars(&mut v);
        self.date.vars(&mut v);
        self.dock.vars(&mut v);
        self.shot.vars(&mut v);
        self.theme.vars(&mut v);
        self.wifi.vars(&mut v);
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

    /// How often each module should be sampled, given what is on screen.
    pub fn polls(&self) -> PollPlan {
        let mut p = PollPlan::new();
        let mut want = |id: ModuleId, ms: Option<u32>| {
            if let Some(ms) = ms {
                p.insert(id, ms);
            }
        };
        want(ModuleId::Audio, self.audio.poll_interval());
        want(ModuleId::Backlight, self.backlight.poll_interval());
        want(ModuleId::Battery, self.battery.poll_interval());
        want(ModuleId::Bluetooth, self.bluetooth.poll_interval());
        want(ModuleId::Date, self.date.poll_interval());
        want(ModuleId::Dock, self.dock.poll_interval());
        want(ModuleId::Theme, self.theme.poll_interval());
        want(ModuleId::Wifi, self.wifi.poll_interval());
        // Fullscreen is event-driven off the compositor, never on a timer.
        p
    }

    fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Bar(e) => self.handle_event_bar(e),
            Event::Clock(e) => self.handle_event_clock(e),
            Event::Audio(e) => self.audio.handle_event(e),
            Event::Backlight(e) => self.backlight.handle_event(e),
            Event::Battery(e) => self.battery.handle_event(e),
            Event::Bluetooth(e) => self.bluetooth.handle_event(e),
            Event::Date(e) => self.date.handle_event(e),
            Event::Dock(e) => self.dock.handle_event(e),
            Event::Shot(e) => self.shot.handle_event(e),
            Event::Theme(e) => self.theme.handle_event(e),
            Event::Wifi(e) => self.wifi.handle_event(e),
        }
    }

    fn handle_event_bar(&mut self, event: &EventBar) {
        match event {
            EventBar::MenuToggled { menu } => {
                if self.menu == Some(*menu) {
                    self.close_menu();
                } else {
                    self.open_menu(*menu);
                }
            }
            EventBar::MenuDismissed => self.close_menu(),
            EventBar::FullscreenDue => self.fullscreen_due = true,
            EventBar::Fullscreen { on } => self.fullscreen = *on,
            EventBar::EwwRestarted => {}
        }
    }

    /// The clock moved. Nothing here decides what that means: the tick goes to
    /// every module that has something to give up on, and each answers from its
    /// own state.
    fn handle_event_clock(&mut self, event: &EventClock) {
        let EventClock::Tick { now } = event;
        debug_assert!(*now >= self.now, "the clock went backwards");
        self.now = *now;
        self.wifi.handle_event(&EventWifi::Tick { now: *now });
        self.bluetooth
            .handle_event(&EventBluetooth::Tick { now: *now });
        self.shot.handle_event(&EventShot::Tick { now: *now });
    }

    /// One pass over every module, collecting what each asks of the bar.
    fn emit_round(&mut self, actions: &mut Vec<Action>) -> Vec<ReqWifi> {
        if std::mem::take(&mut self.fullscreen_due) {
            actions.push(Action::SampleFullscreen);
        }
        let mut reqs = Vec::new();
        // Each module fills its own buffer, and the bar re-labels it on the way
        // out. That one line is the whole cost of a module not knowing there is
        // a bar above it.
        macro_rules! emit {
            ($module:ident, $wrap:expr) => {{
                let mut mine = Vec::new();
                let req = self.$module.emit_actions(&mut mine);
                actions.extend(mine.into_iter().map($wrap));
                req
            }};
        }
        emit!(audio, Action::Audio);
        emit!(backlight, Action::Backlight);
        emit!(battery, Action::Battery);
        emit!(bluetooth, Action::Bluetooth);
        emit!(date, Action::Date);
        emit!(dock, Action::Dock);
        emit!(shot, Action::Shot);
        emit!(theme, Action::Theme);
        reqs.extend(emit!(wifi, Action::Wifi));
        reqs
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

    /// Let the module behind a dropdown know it came into or out of view, so it
    /// can refresh and choose its poll rate.
    fn tell_menu(&mut self, m: MenuId, opened: bool) {
        match m {
            MenuId::Wifi => self.wifi.handle_event(&if opened {
                EventWifi::MenuOpened
            } else {
                EventWifi::MenuClosed
            }),
            MenuId::Bt => self.bluetooth.handle_event(&if opened {
                EventBluetooth::MenuOpened
            } else {
                EventBluetooth::MenuClosed
            }),
            // The sliders have no open/closed state of their own; they just want
            // a fresh value the moment they appear.
            MenuId::Volume => {
                if opened {
                    self.audio.handle_event(&EventAudio::Due)
                }
            }
            MenuId::Brightness => {
                if opened {
                    self.backlight.handle_event(&EventBacklight::Due)
                }
            }
        }
    }
}
