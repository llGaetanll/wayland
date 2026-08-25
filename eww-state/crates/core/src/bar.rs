//! The root state machine.
//!
//! The bar owns the widgets and the one thing none of them can own alone: which
//! dropdown is open. That lives here as a single `Option<MenuId>` — not a
//! boolean per menu — so "two menus open at once" is not a state that exists.
//!
//! Everything else it does is routing. An `Event` comes in, is translated into
//! at most one widget's private event, and the actions that widget raised come
//! out. Widgets that need something of the bar (dismiss me) raise a `Req` and
//! the bar decides.

use std::collections::{BTreeMap, BTreeSet};

use crate::action::{Action, ActionKind};
use crate::config::Config;
use crate::ctx::{Ctx, Req, Vars, WidgetId};
use crate::event::{Event, Intent, JobId, Obs, Probe};
use crate::model::MenuId;
use crate::widgets::{
    audio::{self, Audio},
    backlight::{self, Backlight},
    battery::{self, Battery},
    bluetooth::{self, Bluetooth},
    clock::{self, Clock},
    dock::{self, Dock},
    shot::{self, Shot},
    theme::{self, Theme},
    wifi::{self, Wifi},
    PollPlan, Widget,
};
use crate::window::Window;

pub struct Bar {
    cfg: Config,
    /// Which dropdown is open. The whole point of the type.
    menu: Option<MenuId>,
    /// The focused workspace holds a fullscreen window, so bar and dock hide.
    fullscreen: bool,
    next_job: u64,
    owners: BTreeMap<JobId, WidgetId>,

    pub audio: Audio,
    pub backlight: Backlight,
    pub wifi: Wifi,
    pub bluetooth: Bluetooth,
    pub battery: Battery,
    pub clock: Clock,
    pub theme: Theme,
    pub dock: Dock,
    pub shot: Shot,
}

impl Bar {
    pub fn new(cfg: Config) -> Self {
        Bar {
            cfg,
            menu: None,
            fullscreen: false,
            next_job: 0,
            owners: BTreeMap::new(),
            audio: Audio::default(),
            backlight: Backlight::default(),
            wifi: Wifi::default(),
            bluetooth: Bluetooth::default(),
            battery: Battery::default(),
            clock: Clock::default(),
            theme: Theme::default(),
            dock: Dock::default(),
            shot: Shot::default(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    pub fn menu(&self) -> Option<MenuId> {
        self.menu
    }

    /// Apply one event and collect the side effects it asks for.
    ///
    /// This is the only way state changes. It is deterministic, allocates no
    /// threads, touches no clock and blocks on nothing.
    pub fn handle(&mut self, ev: Event) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut reqs = Vec::new();
        self.route(ev, &mut actions, &mut reqs);

        // A widget may ask the bar for something, and satisfying that may make
        // another widget speak. Bounded so a mistake can't spin forever.
        for _ in 0..8 {
            if reqs.is_empty() {
                break;
            }
            for req in std::mem::take(&mut reqs) {
                match req {
                    Req::CloseMenu => self.close_menu(&mut actions, &mut reqs),
                }
            }
        }
        actions
    }

    /// Every eww variable, as the widgets currently describe themselves.
    ///
    /// Pure, and cheap enough to call after every event: the shell diffs it
    /// against what it last published, so eww only hears about real changes.
    pub fn vars(&self) -> Vars {
        let mut v = Vars::default();
        self.audio.vars(&self.cfg, &mut v);
        self.backlight.vars(&self.cfg, &mut v);
        self.wifi.vars(&self.cfg, &mut v);
        self.bluetooth.vars(&self.cfg, &mut v);
        self.battery.vars(&self.cfg, &mut v);
        self.clock.vars(&self.cfg, &mut v);
        self.theme.vars(&self.cfg, &mut v);
        self.dock.vars(&self.cfg, &mut v);
        self.shot.vars(&self.cfg, &mut v);
        v.set("menu", self.menu.map(|m| m.as_str()).unwrap_or(""));
        v
    }

    /// The windows that should be on screen right now.
    ///
    /// Derived, never remembered: the shell opens and closes to match, so the
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
        self.clock.polls(&self.cfg, &mut p);
        self.theme.polls(&self.cfg, &mut p);
        self.dock.polls(&self.cfg, &mut p);
        self.shot.polls(&self.cfg, &mut p);
        // Fullscreen is event-driven off the compositor, never sampled on a timer.
        p.insert(Probe::Fullscreen, 0);
        p
    }

    fn route(&mut self, ev: Event, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        match ev {
            Event::Intent(i) => self.intent(i, actions, reqs),
            Event::Observed(o) => self.observed(o, actions, reqs),
            Event::JobDone { id, ok, detail } => {
                let Some(owner) = self.owners.remove(&id) else {
                    return; // a job from a previous life; nothing is waiting on it
                };
                match owner {
                    WidgetId::Audio => self.with(WidgetId::Audio, actions, reqs, |b, cx| {
                        b.audio.update(audio::Ev::Done, cx)
                    }),
                    WidgetId::Backlight => {
                        self.with(WidgetId::Backlight, actions, reqs, |b, cx| {
                            b.backlight.update(backlight::Ev::Done, cx)
                        })
                    }
                    WidgetId::Wifi => self.with(WidgetId::Wifi, actions, reqs, |b, cx| {
                        b.wifi.update(wifi::Ev::Done { id, ok, detail }, cx)
                    }),
                    WidgetId::Bluetooth => {
                        self.with(WidgetId::Bluetooth, actions, reqs, |b, cx| {
                            b.bluetooth.update(bluetooth::Ev::Done { id, ok }, cx)
                        })
                    }
                    WidgetId::Battery => self.with(WidgetId::Battery, actions, reqs, |b, cx| {
                        if ok {
                            b.battery.update(battery::Ev::Rendered, cx)
                        }
                    }),
                    WidgetId::Shot => self.with(WidgetId::Shot, actions, reqs, |b, cx| {
                        b.shot.update(shot::Ev::Done { id }, cx)
                    }),
                    WidgetId::Clock | WidgetId::Theme | WidgetId::Dock | WidgetId::Bar => {}
                }
            }
            // Nothing to change: the shell republishes vars and windows from the
            // state we already hold, which is exactly what recovery needs.
            Event::EwwReset => {}
        }
    }

    fn intent(&mut self, i: Intent, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        match i {
            Intent::MenuToggle(m) => {
                if self.menu == Some(m) {
                    self.close_menu(actions, reqs);
                } else {
                    self.open_menu(m, actions, reqs);
                }
            }
            Intent::MenuClose => self.close_menu(actions, reqs),
            Intent::VolumeSet(v) => self.with(WidgetId::Audio, actions, reqs, |b, cx| {
                b.audio.update(audio::Ev::Set(v), cx)
            }),
            Intent::BrightnessSet(v) => self.with(WidgetId::Backlight, actions, reqs, |b, cx| {
                b.backlight.update(backlight::Ev::Set(v), cx)
            }),
            Intent::WifiToggle => self.with(WidgetId::Wifi, actions, reqs, |b, cx| {
                b.wifi.update(wifi::Ev::ToggleRadio, cx)
            }),
            Intent::WifiConnect { ssid } => self.with(WidgetId::Wifi, actions, reqs, |b, cx| {
                b.wifi.update(wifi::Ev::Connect { ssid }, cx)
            }),
            Intent::BtToggle => self.with(WidgetId::Bluetooth, actions, reqs, |b, cx| {
                b.bluetooth.update(bluetooth::Ev::TogglePower, cx)
            }),
            Intent::BtConnect { mac } => self.with(WidgetId::Bluetooth, actions, reqs, |b, cx| {
                b.bluetooth.update(bluetooth::Ev::ToggleDevice { mac }, cx)
            }),
            Intent::DockLaunch { id } => self.with(WidgetId::Dock, actions, reqs, |b, cx| {
                b.dock.update(dock::Ev::Launch { id }, cx)
            }),
            Intent::DockSwap { a, b: other } => {
                self.with(WidgetId::Dock, actions, reqs, |b, cx| {
                    b.dock.update(dock::Ev::Swap { a, b: other }, cx)
                })
            }
            Intent::ShotCapture => self.with(WidgetId::Shot, actions, reqs, |b, cx| {
                b.shot.update(shot::Ev::Capture, cx)
            }),
            Intent::ShotStage(s) => self.with(WidgetId::Shot, actions, reqs, |b, cx| {
                b.shot.update(shot::Ev::Staged(s), cx)
            }),
            Intent::ShotAct(a) => self.with(WidgetId::Shot, actions, reqs, |b, cx| {
                b.shot.update(shot::Ev::Act(a), cx)
            }),
            Intent::Refresh(p) => self.with(WidgetId::Bar, actions, reqs, |_, cx| {
                cx.dispatch(ActionKind::Probe(p));
            }),
        }
    }

    fn observed(&mut self, o: Obs, actions: &mut Vec<Action>, reqs: &mut Vec<Req>) {
        match o {
            Obs::Theme(t) => self.with(WidgetId::Theme, actions, reqs, |b, cx| {
                b.theme.update(theme::Ev::Observed(t), cx)
            }),
            Obs::Clock(t) => self.with(WidgetId::Clock, actions, reqs, |b, cx| {
                b.clock.update(clock::Ev::Observed(t), cx)
            }),
            Obs::Battery(l) => self.with(WidgetId::Battery, actions, reqs, |b, cx| {
                b.battery.update(battery::Ev::Observed(l), cx)
            }),
            Obs::Audio(a) => self.with(WidgetId::Audio, actions, reqs, |b, cx| {
                b.audio.update(audio::Ev::Observed(a), cx)
            }),
            Obs::Backlight(v) => self.with(WidgetId::Backlight, actions, reqs, |b, cx| {
                b.backlight.update(backlight::Ev::Observed(v), cx)
            }),
            Obs::Net(n) => self.with(WidgetId::Wifi, actions, reqs, |b, cx| {
                b.wifi.update(wifi::Ev::Observed(n), cx)
            }),
            Obs::Bt(s) => self.with(WidgetId::Bluetooth, actions, reqs, |b, cx| {
                b.bluetooth.update(bluetooth::Ev::Observed(s), cx)
            }),
            Obs::Dock(apps) => self.with(WidgetId::Dock, actions, reqs, |b, cx| {
                b.dock.update(dock::Ev::Observed(apps), cx)
            }),
            Obs::Fullscreen(f) => self.fullscreen = f,
        }
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

    /// Let the widget behind a dropdown know it came into or out of view, so it
    /// can refresh and choose its poll rate.
    fn tell_menu(
        &mut self,
        m: MenuId,
        opened: bool,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
    ) {
        match m {
            MenuId::Wifi => self.with(WidgetId::Wifi, actions, reqs, |b, cx| {
                let ev = if opened {
                    wifi::Ev::MenuOpened
                } else {
                    wifi::Ev::MenuClosed
                };
                b.wifi.update(ev, cx)
            }),
            MenuId::Bt => self.with(WidgetId::Bluetooth, actions, reqs, |b, cx| {
                let ev = if opened {
                    bluetooth::Ev::MenuOpened
                } else {
                    bluetooth::Ev::MenuClosed
                };
                b.bluetooth.update(ev, cx)
            }),
            // The sliders have no open/closed state of their own; they just want
            // a fresh value the moment they appear.
            MenuId::Volume => {
                if opened {
                    self.with(WidgetId::Audio, actions, reqs, |_, cx| {
                        cx.dispatch(ActionKind::Probe(Probe::Audio));
                    })
                }
            }
            MenuId::Brightness => {
                if opened {
                    self.with(WidgetId::Backlight, actions, reqs, |_, cx| {
                        cx.dispatch(ActionKind::Probe(Probe::Backlight));
                    })
                }
            }
        }
    }

    /// Run `f` with a `Ctx` that attributes anything it dispatches to `owner`.
    fn with(
        &mut self,
        owner: WidgetId,
        actions: &mut Vec<Action>,
        reqs: &mut Vec<Req>,
        f: impl FnOnce(&mut Bar, &mut Ctx),
    ) {
        // Split the borrow: the closure gets the widgets, the Ctx gets the
        // bookkeeping. Taking them out and putting them back is the price of
        // keeping widget code free of any reference to the bar.
        let mut next_job = self.next_job;
        let mut owners = std::mem::take(&mut self.owners);
        {
            let mut cx = Ctx::new(owner, &mut next_job, actions, &mut owners, reqs);
            f(self, &mut cx);
        }
        self.next_job = next_job;
        self.owners = owners;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NetSnapshot, Shot as Staged, ShotAction, WifiNetwork};

    fn bar() -> Bar {
        Bar::new(Config::new("/i", "/c"))
    }

    fn names(b: &Bar) -> Vec<String> {
        b.windows().into_iter().map(|w| w.name()).collect()
    }

    #[test]
    fn only_one_menu_can_be_open() {
        let mut b = bar();
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Wifi)));
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Bt)));
        assert_eq!(b.menu(), Some(MenuId::Bt));
        let open = names(&b);
        assert!(open.contains(&"bt-menu".to_string()));
        assert!(!open.contains(&"wifi-menu".to_string()));
        assert!(open.contains(&"menu-backdrop".to_string()));
    }

    #[test]
    fn toggling_the_open_menu_closes_it() {
        let mut b = bar();
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Volume)));
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Volume)));
        assert_eq!(b.menu(), None);
        assert!(!names(&b).iter().any(|n| n.ends_with("-menu")));
        assert!(!names(&b).contains(&"menu-backdrop".to_string()));
    }

    #[test]
    fn the_screenshot_preview_is_independent_of_bar_menus() {
        let mut b = bar();
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Wifi)));
        b.handle(Event::Intent(Intent::ShotStage(Staged {
            path: "/a.png".into(),
            width: 100,
            height: 50,
        })));
        assert_eq!(
            b.menu(),
            Some(MenuId::Wifi),
            "a capture leaves the menu alone"
        );
        assert!(names(&b).contains(&"shot-menu".to_string()));

        b.handle(Event::Intent(Intent::ShotAct(ShotAction::Dismiss)));
        assert_eq!(b.menu(), Some(MenuId::Wifi));
        assert!(!names(&b).contains(&"shot-menu".to_string()));
    }

    #[test]
    fn a_successful_join_dismisses_the_menu() {
        let mut b = bar();
        b.handle(Event::Observed(Obs::Net(NetSnapshot {
            enabled: true,
            connected: None,
            networks: vec![WifiNetwork {
                ssid: "cafe".into(),
                signal: 70,
                secure: true,
                active: false,
            }],
            wired: false,
        })));
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Wifi)));
        let acts = b.handle(Event::Intent(Intent::WifiConnect {
            ssid: "cafe".into(),
        }));
        let id = acts
            .iter()
            .find(|a| matches!(a.kind, ActionKind::WifiConnect { .. }))
            .expect("dispatched a join")
            .id;
        b.handle(Event::JobDone {
            id,
            ok: true,
            detail: None,
        });
        assert_eq!(b.menu(), None, "the widget asked the bar to dismiss it");
    }

    #[test]
    fn fullscreen_hides_bar_and_dock_together() {
        let mut b = bar();
        assert_eq!(names(&b), ["bar", "dock"]);
        b.handle(Event::Observed(Obs::Fullscreen(true)));
        assert!(names(&b).is_empty());
        // A menu is still reachable from a keybind while the bar is hidden.
        b.handle(Event::Intent(Intent::MenuToggle(MenuId::Volume)));
        assert_eq!(names(&b), ["volume-menu", "menu-backdrop"]);
        b.handle(Event::Observed(Obs::Fullscreen(false)));
        assert_eq!(names(&b), ["bar", "dock", "volume-menu", "menu-backdrop"]);
    }

    #[test]
    fn a_probe_result_does_not_ask_for_another_probe() {
        let mut b = bar();
        let acts = b.handle(Event::Intent(Intent::Refresh(Probe::Audio)));
        let id = acts[0].id;
        assert!(
            b.handle(Event::JobDone {
                id,
                ok: true,
                detail: None
            })
            .is_empty(),
            "a probe finishing must not start another one"
        );
    }

    #[test]
    fn a_job_from_a_previous_life_is_ignored() {
        let mut b = bar();
        let before = b.vars();
        b.handle(Event::JobDone {
            id: JobId(999),
            ok: true,
            detail: None,
        });
        assert_eq!(b.vars(), before);
    }

    #[test]
    fn opening_a_menu_refreshes_what_it_shows() {
        let mut b = bar();
        let acts = b.handle(Event::Intent(Intent::MenuToggle(MenuId::Bt)));
        assert!(acts.iter().any(|a| a.kind == ActionKind::Probe(Probe::Bt)));
        let acts = b.handle(Event::Intent(Intent::MenuToggle(MenuId::Volume)));
        assert!(acts
            .iter()
            .any(|a| a.kind == ActionKind::Probe(Probe::Audio)));
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
