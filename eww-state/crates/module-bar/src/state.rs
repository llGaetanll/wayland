//! The root state machine.
//!
//! The bar owns the modules and the two facts none of them can own alone: which
//! dropdown is open, and which eww server we are talking to. The first lives
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
use module_battery::EventBattery;
use module_battery::StateBattery;
use module_bluetooth::EventBluetooth;
use module_bluetooth::StateBluetooth;
use module_date::StateDate;
use module_dock::StateDock;
use module_search::StateSearch;
use module_shot::EventShot;
use module_shot::StateShot;
use module_spaces::EventSpaces;
use module_spaces::StateSpaces;
use module_theme::StateTheme;
use module_theme::Theme;
use module_wifi::EventWifi;
use module_wifi::ReqWifi;
use module_wifi::StateWifi;

use crate::action::Action;
use crate::action_bar::ActionBar;
use crate::config::ConfigBar;
use crate::event::Event;
use crate::event_bar::EventBar;
use crate::event_clock::EventClock;
use crate::eww_gen::EwwGen;
use crate::menu::MenuId;
use crate::module_id::ModuleId;
use crate::module_id::PollPlan;
use crate::server_id::ServerId;
use crate::window::Window;

/// Granting a module's request can make another module speak, whose request can
/// in turn... Bounded so a mistake cannot spin forever.
const MAX_EMIT_ROUNDS: usize = 8;

/// How long a server we asked for has to appear before we ask again.
///
/// Both ends of this matter. Asking again too soon races a server that is still
/// coming up, and a second server is the failure this whole path exists to
/// prevent. Never asking again leaves an empty screen for the rest of the
/// session when a start quietly fails.
const EWW_START_AGAIN: Millis = Millis(10_000);

/// How long a stamp has to be on the wire before a table that is not carrying
/// it counts as reset.
///
/// The probe samples on its own thread, so a read can be taken before a stamp
/// is published and still arrive after it. Without this the answer to that read
/// would be to stamp again, which the next stale read would answer the same
/// way, for ever. Longer than the probe interval, so the window always contains
/// a fresh read; on a real reload it is what the healing costs.
const EWW_STAMP_SETTLE: Millis = Millis(10_000);

/// The eww var the generation is stamped into. Declared in `eww.yuck` as
/// `(defvar eww_gen "")`, and read back by the probe.
pub const EWW_GEN_VAR: &str = "eww_gen";

#[derive(Default)]
pub struct StateBar {
    /// Which dropdown is open. The whole point of the type.
    menu: Option<MenuId>,
    now: Millis,
    /// The eww server the last probe saw, and the one we have already published
    /// to. Two fields and not one, because the difference between them is the
    /// action: a server we have seen but not adopted is one that knows nothing
    /// about us yet, whether it has just started or has just replaced another.
    eww_seen: Option<ServerId>,
    eww_adopted: Option<ServerId>,
    /// When we last asked for a server, so a run of empty observations asks
    /// once rather than once every probe: asking twice is how a second server
    /// gets started while the first is still coming up.
    eww_started: Option<Millis>,
    /// The generation we are publishing, and the one the last probe read back
    /// out of eww. Two fields for the same reason the server is two: the
    /// difference between them is a table that has to be filled again.
    eww_gen: EwwGen,
    eww_gen_seen: Option<EwwGen>,
    /// When the current generation was stamped, so a read taken before it
    /// landed is not mistaken for the table being thrown away a second time.
    eww_stamped: Option<Millis>,

    audio: StateAudio,
    backlight: StateBacklight,
    battery: StateBattery,
    bluetooth: StateBluetooth,
    date: StateDate,
    dock: StateDock,
    search: StateSearch,
    shot: StateShot,
    spaces: StateSpaces,
    theme: StateTheme,
    wifi: StateWifi,
    /// Where the two icon sets live, and which one the modules are currently
    /// pointed at. macOS draws its menu bar glyphs white in dark appearance and
    /// black in light, and the glyphs here are one colour baked into the file,
    /// so following the theme means swapping directories.
    icons_root: String,
    icons_theme: Option<Theme>,
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
            search: StateSearch::new(cfg.search),
            shot: StateShot::new(cfg.shot),
            spaces: StateSpaces::new(cfg.spaces),
            theme: StateTheme::new(cfg.theme),
            wifi: StateWifi::new(cfg.wifi),
            icons_root: cfg.icons_root,
            ..StateBar::default()
        }
    }

    /// Point every glyph-drawing module at the icon set for the current
    /// appearance. Idempotent, and does nothing until the theme actually
    /// changes, so it is safe to call after every batch.
    fn follow_theme(&mut self) {
        let mode = self.theme.mode();
        if self.icons_theme == Some(mode) {
            return;
        }
        self.icons_theme = Some(mode);
        let dir = format!("{}/{}", self.icons_root, mode.as_str());
        self.audio.set_icons_dir(&dir);
        self.backlight.set_icons_dir(&dir);
        self.bluetooth.set_icons_dir(&dir);
        self.wifi.set_icons_dir(&dir);
        // The battery glyph is generated rather than picked off disk, so it is
        // told the appearance instead of a directory.
        self.battery.set_dark(mode == Theme::Dark);
    }

    /// Let the placement module see the dock's apps, so a minimized window's
    /// tile can be badged with the icon of the app it belongs to.
    ///
    /// The dock's list is the dock's own; this hands down a copy rather than
    /// letting one module read into another, which is the only direction
    /// anything here is allowed to speak in.
    fn follow_dock(&mut self) {
        let icons: Vec<(String, String)> = self
            .dock
            .apps()
            .iter()
            .map(|a| (a.id.clone(), a.icon.clone()))
            .collect();
        self.spaces.set_app_icons(icons.clone());
        // The energy list names apps, and an app that is one of these should be
        // drawn with its picture rather than a generic glyph.
        self.battery.set_app_icons(icons);
        // And which window belongs to which process, so CPU time can be
        // attributed to the app that owns it. Both copies come from the module
        // that owns the fact, which is the only direction anything here speaks.
        self.battery.set_window_owners(self.spaces.window_owners());
    }

    /// Apply a batch of events. This is the only way state changes, and it
    /// emits nothing: what the batch adds up to is not known until it is over.
    pub fn handle_events(&mut self, events: &[Event]) {
        for event in events {
            self.handle_event(event);
        }
        self.follow_theme();
        self.follow_dock();
    }

    /// Say what has to happen now, given everything the batch left behind.
    pub fn emit_actions(&mut self, actions: &mut Vec<Action>) {
        self.emit_eww(actions);
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

    pub fn spaces(&self) -> &StateSpaces {
        &self.spaces
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
        self.search.vars(&mut v);
        self.shot.vars(&mut v);
        self.spaces.vars(&mut v);
        self.theme.vars(&mut v);
        self.wifi.vars(&mut v);
        v.set("menu", self.menu.map(|m| m.as_str()).unwrap_or(""));
        // Published like any other var, so it travels the same diff and is
        // re-sent with everything else the moment eww is adopted.
        v.set(EWW_GEN_VAR, self.eww_gen);
        v
    }

    /// The windows that should be on screen right now.
    ///
    /// Derived, never remembered: the runtime opens and closes to match, so the
    /// var and the actual surfaces cannot drift apart the way they did when
    /// each script opened its own.
    pub fn windows(&self) -> BTreeSet<Window> {
        let mut w = BTreeSet::new();
        // A fullscreen Space is the compositor's business, so the module that
        // owns Spaces is asked rather than a copy of the answer kept here.
        if !self.spaces.fullscreen() {
            w.insert(Window::Bar);
            // Which dock: the low one peeks out from under a tiled window
            // instead of covering its bottom edge. Asked of the module that
            // owns the layout, for the same reason fullscreen is.
            w.insert(if self.spaces.tiled() {
                Window::DockLow
            } else {
                Window::Dock
            });
        }
        if let Some(m) = self.menu {
            w.insert(Window::Menu(m));
            w.insert(Window::MenuBackdrop);
        }
        if self.shot.is_open() {
            w.insert(Window::Shot);
            w.insert(Window::ShotBackdrop);
        }
        if self.search.is_open() {
            w.insert(Window::Search);
            w.insert(Window::SearchBackdrop);
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
        want(ModuleId::Spaces, self.spaces.poll_interval());
        want(ModuleId::Theme, self.theme.poll_interval());
        want(ModuleId::Wifi, self.wifi.poll_interval());
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
            Event::Search(e) => self.search.handle_event(e),
            Event::Shot(e) => self.shot.handle_event(e),
            Event::Spaces(e) => self.spaces.handle_event(e),
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
            EventBar::EwwObserved { server, gen } => {
                self.eww_seen = *server;
                self.eww_gen_seen = *gen;
            }
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

    /// Keep who we are publishing to and who is actually listening in step.
    ///
    /// An eww client that cannot connect forks itself into a server rather than
    /// failing, and that server rebinds the socket — so the one we are talking
    /// to can be replaced without anything dying and without anything asking
    /// us. A server we have not adopted holds none of our variables and none of
    /// our windows, so it is told everything again.
    ///
    /// The same server can lose everything too: `eww reload` re-reads the
    /// config and puts every var back at its default, on the same socket and
    /// the same inode. That is what the generation is for — an adoption is a
    /// server we have not published to, or a table that is no longer carrying
    /// our stamp, and the remedy is identical either way.
    fn emit_eww(&mut self, actions: &mut Vec<Action>) {
        match self.eww_seen {
            None => {
                self.eww_adopted = None;
                let due = match self.eww_started {
                    None => true,
                    Some(asked) => self.now.since(asked) >= EWW_START_AGAIN,
                };
                if due {
                    self.eww_started = Some(self.now);
                    actions.push(Action::Bar(ActionBar::EwwStart));
                }
            }
            Some(server) => {
                self.eww_started = None;
                if self.eww_adopted != Some(server) || self.eww_forgot_us() {
                    self.eww_adopted = Some(server);
                    self.eww_gen = self.eww_gen.next();
                    self.eww_stamped = Some(self.now);
                    actions.push(Action::Bar(ActionBar::EwwAdopt { server }));
                }
            }
        }
    }

    /// Whether the server we are already publishing to is holding a variable
    /// table that is not the one we filled.
    ///
    /// Only a read from after the current stamp was published can say: an older
    /// generation, the unstamped default included, is what a read still in
    /// flight when we stamped would carry, and answering that by stamping again
    /// is a loop with no end. See [`EWW_STAMP_SETTLE`].
    fn eww_forgot_us(&self) -> bool {
        if self.eww_gen_seen == Some(self.eww_gen) {
            return false;
        }
        match self.eww_stamped {
            None => true,
            Some(at) => self.now.since(at) >= EWW_STAMP_SETTLE,
        }
    }

    /// One pass over every module, collecting what each asks of the bar.
    fn emit_round(&mut self, actions: &mut Vec<Action>) -> Vec<ReqWifi> {
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
        emit!(search, Action::Search);
        emit!(shot, Action::Shot);
        emit!(spaces, Action::Spaces);
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
            // The Sound panel has a list in it, so unlike Display it cares
            // which way it went: the outputs are only sampled while it is up.
            MenuId::Volume => self.audio.handle_event(&if opened {
                EventAudio::MenuOpened
            } else {
                EventAudio::MenuClosed
            }),
            MenuId::Brightness => {
                if opened {
                    self.backlight.handle_event(&EventBacklight::Due)
                }
            }
            // Both halves of the Battery panel are expensive and neither is
            // worth anything off screen, so like Sound it cares which way it
            // went rather than only that it was asked.
            MenuId::Battery => {
                self.battery.handle_event(&if opened {
                    EventBattery::MenuOpened
                } else {
                    EventBattery::MenuClosed
                });
                // The energy list is attributed through the compositor's window
                // list, and Spaces is event-driven — it holds whatever the last
                // compositor signal left behind, which after a daemon restart
                // mid-session is nothing at all. Opening the panel is the one
                // moment that list has to be current, so it is asked for.
                if opened {
                    self.spaces.handle_event(&EventSpaces::Due);
                }
            }
        }
    }
}
