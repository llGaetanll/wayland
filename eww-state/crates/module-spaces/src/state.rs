//! Where every window lives. No IO.
//!
//! Two rules share this machine because they are one fact: a window is in a
//! fullscreen Space, or minimized in the dock, or at home, and never two of
//! those. Split across two modules each would keep its own record of where a
//! window came from, each would move windows, and each would have to absorb the
//! other's moves coming back off the compositor as if the user had made them.
//!
//! Every rule here is decided against a *settled* look at the compositor rather
//! than against the event that woke us, which is what removed the timers this
//! logic used to need.
//!
//! The trap it exists for: moving a fullscreen window makes the compositor
//! re-emit fullscreen — a transient `0` followed by a `1` — and that `0` reads
//! exactly like the user leaving fullscreen. Act on it and the window is moved
//! home, which emits the echo again, forever. The Lua version swallowed those
//! with a 140ms debounce and hoped.
//!
//! They need no debounce, because the echoes are ours: the module knows it has
//! a move in flight, and a look taken while one is cannot be believed. So a
//! move suspends every rule until it finishes, and the first look afterwards is
//! the first one trusted. Nothing here reads a clock.

use std::collections::BTreeMap;

use api::jobs::JobId;
use api::jobs::Jobs;
use api::module::Module;
use api::util::class_matches;
use api::wire_json::WireJson;
use api::wire_vars::WireVars;

use crate::action::ActionSpaces;
use crate::config::ConfigSpaces;
use crate::event::EventSpaces;
use crate::model::Minimized;
use crate::model::MoveTarget;
use crate::model::Rect;
use crate::model::Snapshot;
use crate::model::WindowAddr;
use crate::model::WorkspaceId;

#[derive(Debug, Default)]
pub struct StateSpaces {
    cfg: ConfigSpaces,
    jobs: Jobs,
    /// Where each window in a Space came from. Absent means *not in a Space* —
    /// this map is the whole of what used to be a directory of files, and being
    /// a map it cannot go stale behind a window's back.
    origins: BTreeMap<WindowAddr, WorkspaceId>,
    /// The last look worth believing.
    layout: Snapshot,
    /// Moves we asked for and have not seen finish, by the job carrying each.
    moving: BTreeMap<JobId, WindowAddr>,
    /// Looks older than this were taken while a move of ours was in flight, so
    /// what they say about fullscreen is the compositor echoing us back.
    /// `None` until a move has ever happened, when every look is honest.
    trust_from: Option<JobId>,
    /// Windows put away in the dock, oldest first — the order the tiles are
    /// drawn in, which is why this is a list and not a map. macOS adds each
    /// minimized window at the right-hand end and leaves the rest where they
    /// are; sorting by address instead would reshuffle the dock every time
    /// anything was minimized.
    minimized: Vec<Minimized>,
    /// The dock's apps, as `(id, icon)`, for badging a thumbnail with the icon
    /// of the app it belongs to. Handed down rather than read: which apps are
    /// in the dock is not this module's fact.
    icons: Vec<(String, String)>,
    /// Captures taken this session, so no two thumbnails share a path. GTK
    /// caches a `background-image` by its path, so a reused name would draw the
    /// window that was minimized before this one.
    captures: u64,
    /// Thumbnails with nothing left to show, waiting to be deleted.
    forgetting: Vec<String>,
    /// A move finished, so the next look is the one to start trusting.
    settling: bool,
    /// ...and until that look lands, the layout in hand is one from before the
    /// move, so no rule may be decided on it. Distinct from `settling`, which
    /// is consumed by the look going out; this is only cleared by an answer.
    unsettled: bool,
    /// A look the batch asked for, collapsed: a burst of compositor events is
    /// one question.
    due: bool,
    /// A look already in flight, so a burst does not become a burst of them.
    looking: Option<JobId>,
    closing: Option<Awaiting>,
    minimizing: Option<Awaiting>,
    /// Tiles clicked this batch. Two clicks on the same tile are one restore,
    /// and the second is dropped when the first takes the window off the list.
    restoring: Vec<WindowAddr>,
}

/// Something asked of the focused window, which takes two turns: which window
/// has focus is the one thing a look taken before the click can't be trusted
/// about, so a fresh one is sent for and the answer waited on.
#[derive(Clone, Copy, Debug)]
enum Awaiting {
    /// The click landed.
    Wanted,
    /// A look was sent for it; this is the one to wait for.
    Look(JobId),
    /// It arrived, so the focused window is known.
    Ready,
}

impl StateSpaces {
    pub fn new(cfg: ConfigSpaces) -> StateSpaces {
        StateSpaces {
            cfg,
            ..StateSpaces::default()
        }
    }

    /// Point thumbnail badges at the dock's apps, as `(id, icon)` pairs.
    ///
    /// Idempotent and cheap; the parent calls it after every batch, because the
    /// dock's list is the dock's to change and this module is only a reader of
    /// it.
    pub fn set_app_icons(&mut self, icons: Vec<(String, String)>) {
        self.icons = icons;
    }

    pub fn minimized(&self) -> &[Minimized] {
        &self.minimized
    }

    /// The focused workspace holds a fullscreen window, so the bar and dock
    /// should get out of the way.
    ///
    /// Derived from the same look the Space rules are decided on, so what is on
    /// screen and where windows are cannot disagree about what the compositor
    /// said.
    pub fn fullscreen(&self) -> bool {
        self.layout.fullscreen()
    }

    /// The focused workspace holds a tiled window, so the dock should drop to
    /// the peek it keeps below the screen rather than sit on top of that
    /// window's bottom edge.
    ///
    /// Derived from the same look as everything else here, so the dock cannot
    /// be told a layout the Space rules disagree with.
    pub fn tiled(&self) -> bool {
        self.layout.tiled()
    }

    /// This window is in a Space of its own.
    pub fn in_space(&self, addr: &WindowAddr) -> bool {
        self.origins.contains_key(addr)
    }

    /// Ask the compositor what it looks like now.
    fn look(&mut self, actions: &mut Vec<ActionSpaces>) -> JobId {
        let id = self.jobs.take();
        debug_assert!(
            self.trust_from.is_none_or(|t| id >= t),
            "job ids went backwards"
        );
        if std::mem::take(&mut self.settling) {
            self.trust_from = Some(id);
        }
        self.looking = Some(id);
        actions.push(ActionSpaces::Look { id });
        id
    }

    fn observed(&mut self, id: JobId, snapshot: &Snapshot) {
        // An answer is proof the look is over, believed or not — otherwise a
        // look dropped for arriving mid-move would block every later one.
        if self.looking == Some(id) {
            self.looking = None;
        }
        // Mid-cut, or taken while it was: what the compositor says about
        // fullscreen during a move of ours is the move, not the user.
        if !self.moving.is_empty() || self.trust_from.is_some_and(|t| id < t) {
            return;
        }
        self.closing = arrived(self.closing, id);
        self.minimizing = arrived(self.minimizing, id);
        self.layout = snapshot.clone();
        self.unsettled = false;
        // A window that has gone takes its origin with it. This is the pruning
        // sweep the shell version ran over a directory, except that there is
        // nothing left behind to sweep.
        self.origins
            .retain(|addr, _| snapshot.client(addr).is_some());
        // ...and its thumbnail, which unlike an origin is a file on disk, so
        // the sweep has something to hand back.
        let mut stale = Vec::new();
        self.minimized.retain(|m| {
            let alive = snapshot.client(&m.addr).is_some();
            if !alive {
                stale.push(m.thumb.clone());
            }
            alive
        });
        self.forgetting.append(&mut stale);
    }

    fn finished(&mut self, id: JobId) {
        if self.looking == Some(id) {
            self.looking = None;
            // A look that produced nothing cannot be waited on forever.
            if matches!(self.closing, Some(Awaiting::Look(w)) if w == id) {
                self.closing = None;
            }
            if matches!(self.minimizing, Some(Awaiting::Look(w)) if w == id) {
                self.minimizing = None;
            }
        }
        if self.moving.remove(&id).is_some() && self.moving.is_empty() {
            self.settling = true;
            self.unsettled = true;
            self.due = true;
        }
    }

    /// Put every window where the rule says it belongs.
    ///
    /// Skipped entirely while a move of ours is in flight, and until the first
    /// look after one lands: the compositor is mid-cut, a `Sampled` from then
    /// is dropped rather than acted on, and what is left in hand is a picture
    /// of the world from before the move. Deciding on that is what sent a
    /// window the user had just restored straight back out again — the layout
    /// still had it hidden, and nothing remembered minimizing it any more.
    fn settle(&mut self, actions: &mut Vec<ActionSpaces>) {
        if !self.moving.is_empty() || self.unsettled {
            return;
        }
        let mut entering = Vec::new();
        let mut leaving = Vec::new();
        let mut stranded = Vec::new();
        for client in &self.layout.clients {
            // A minimized window is outside the Space rules entirely. It is not
            // fullscreen and it is not where it lives, and read by the rules
            // below that is a window to put right — which would pull it back
            // out of the dock the moment it went in.
            if self.is_minimized(&client.addr) {
                continue;
            }
            // Hidden, with nobody remembering it: a daemon that died took the
            // only record of where it came from, and nothing on screen can ask
            // for it back. It is stranded until something here notices.
            if client.workspace_name == self.cfg.hidden {
                stranded.push(client.addr.clone());
                continue;
            }
            match (client.fullscreen, self.origins.contains_key(&client.addr)) {
                (true, false) => entering.push((client.addr.clone(), client.workspace)),
                (false, true) => leaving.push(client.addr.clone()),
                // Already in a Space, or an ordinary window that was never in
                // one. Both are the common case and both are nothing to do.
                (true, true) | (false, false) => {}
            }
        }
        for (addr, from) in entering {
            self.origins.insert(addr.clone(), from);
            self.dispatch_move(addr, MoveTarget::Empty, actions);
        }
        for addr in leaving {
            if let Some(workspace) = self.origins.remove(&addr) {
                self.dispatch_move(addr, MoveTarget::Workspace(workspace), actions);
            }
        }
        for addr in stranded {
            let home = self.cfg.fallback_home;
            tracing::info!(%addr, %home, "rescuing a window left hidden by a previous run");
            self.dispatch_move(addr, MoveTarget::Workspace(home), actions);
        }
    }

    fn is_minimized(&self, addr: &WindowAddr) -> bool {
        self.minimized.iter().any(|m| &m.addr == addr)
    }

    /// Put the focused window away in the dock.
    ///
    /// Silent about every reason it might not: a window that has gone between
    /// the click and the look, and a second click on one already minimized, are
    /// both the user being faster than the compositor rather than anything to
    /// report.
    fn minimize(&mut self, actions: &mut Vec<ActionSpaces>) {
        let Some(addr) = self.layout.active.clone() else {
            return;
        };
        // macOS greys the yellow button out on a fullscreen window, and this is
        // the same rule from the other side: a window minimized out of a Space
        // would leave the Space behind it, and the rule that brings a window
        // home would spend the rest of the session pulling against the one that
        // keeps it hidden.
        if self.origins.contains_key(&addr) || self.is_minimized(&addr) {
            return;
        }
        let Some(client) = self.layout.client(&addr) else {
            return;
        };
        self.captures += 1;
        let thumb = format!(
            "{}/{}-{}.png",
            self.cfg.thumbs,
            addr.as_str().trim_start_matches("0x"),
            self.captures
        );
        let geometry = client.geometry;
        self.minimized.push(Minimized {
            addr: addr.clone(),
            home: client.workspace,
            title: client.title.clone(),
            thumb: thumb.clone(),
            captured: false,
            width: tile_width(geometry, self.cfg.tile_height),
            icon: self.icon_for(&client.class),
        });
        let id = self.jobs.take();
        self.moving.insert(id, addr.clone());
        actions.push(ActionSpaces::Minimize {
            id,
            addr,
            geometry,
            thumb,
            to: MoveTarget::Hidden(self.cfg.hidden.clone()),
        });
    }

    /// Bring a window back out of the dock, to the workspace it left from.
    fn restore(&mut self, addr: &WindowAddr, actions: &mut Vec<ActionSpaces>) {
        let Some(at) = self.minimized.iter().position(|m| &m.addr == addr) else {
            return;
        };
        let entry = self.minimized.remove(at);
        self.dispatch_move(
            entry.addr.clone(),
            MoveTarget::Workspace(entry.home),
            actions,
        );
        actions.push(ActionSpaces::Forget { thumb: entry.thumb });
    }

    /// The dock icon of the app a window class belongs to, or nothing.
    fn icon_for(&self, class: &str) -> String {
        self.icons
            .iter()
            .find(|(id, _)| class_matches(id, class))
            .map(|(_, icon)| icon.clone())
            .unwrap_or_default()
    }

    fn dispatch_move(&mut self, addr: WindowAddr, to: MoveTarget, actions: &mut Vec<ActionSpaces>) {
        let id = self.jobs.take();
        self.moving.insert(id, addr.clone());
        actions.push(ActionSpaces::Move { id, addr, to });
    }
}

impl Module for StateSpaces {
    type Event = EventSpaces;
    type Action = ActionSpaces;
    type Req = ();

    fn handle_event(&mut self, event: &EventSpaces) {
        match event {
            EventSpaces::Due => self.due = true,
            EventSpaces::CloseRequested => self.closing = Some(Awaiting::Wanted),
            EventSpaces::MinimizeRequested => self.minimizing = Some(Awaiting::Wanted),
            EventSpaces::RestoreRequested { addr } => {
                if !self.restoring.contains(addr) {
                    self.restoring.push(addr.clone());
                }
            }
            EventSpaces::Captured { addr } => {
                if let Some(m) = self.minimized.iter_mut().find(|m| &m.addr == addr) {
                    m.captured = true;
                }
            }
            EventSpaces::Sampled { id, snapshot } => self.observed(*id, snapshot),
            EventSpaces::Finished { id } => self.finished(*id),
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionSpaces>) -> Option<()> {
        // Each of these sends for a look of its own and deliberately does not
        // reuse one already in flight: that one was sent for before the click,
        // and could name the window the user was on a moment ago.
        if matches!(self.closing, Some(Awaiting::Wanted)) {
            let id = self.look(actions);
            self.closing = Some(Awaiting::Look(id));
        }
        if matches!(self.minimizing, Some(Awaiting::Wanted)) {
            let id = self.look(actions);
            self.minimizing = Some(Awaiting::Look(id));
        }
        if matches!(self.closing, Some(Awaiting::Ready)) {
            self.closing = None;
            if let Some(addr) = self.layout.active.clone() {
                // The origin stays recorded until the window is actually gone,
                // and the next look prunes it. Dropping it here would leave a
                // window that is still fullscreen with no Space of its own,
                // and the rules below would put it in a new one on the way out.
                if let Some(workspace) = self.origins.get(&addr).copied() {
                    actions.push(ActionSpaces::Focus { workspace });
                }
                actions.push(ActionSpaces::Close { addr });
            }
        }
        if matches!(self.minimizing, Some(Awaiting::Ready)) {
            self.minimizing = None;
            self.minimize(actions);
        }
        for addr in std::mem::take(&mut self.restoring) {
            self.restore(&addr, actions);
        }
        for thumb in std::mem::take(&mut self.forgetting) {
            actions.push(ActionSpaces::Forget { thumb });
        }
        if std::mem::take(&mut self.due) && self.looking.is_none() {
            self.look(actions);
        }
        self.settle(actions);
        None
    }

    /// The minimized windows, in the order the dock draws them.
    ///
    /// The only thing here eww is told directly. That a window is in a Space is
    /// still not published: what eww does about it — hide the bar and dock — is
    /// the bar's decision, taken from [`fullscreen`](Self::fullscreen).
    fn vars(&self, out: &mut WireVars) {
        let tiles: Vec<WireJson> = self
            .minimized
            .iter()
            .map(|m| {
                WireJson::Obj(vec![
                    ("addr", WireJson::s(m.addr.as_str())),
                    // A tile whose photograph has not landed yet says so with
                    // an empty path, which draws nothing. It is still a tile,
                    // and clicking it still brings the window back.
                    (
                        "thumb",
                        WireJson::s(if m.captured { m.thumb.as_str() } else { "" }),
                    ),
                    ("width", WireJson::Int(m.width.into())),
                    ("icon", WireJson::s(&m.icon)),
                    ("title", WireJson::s(&m.title)),
                ])
            })
            .collect();
        out.set("dock_minimized", WireJson::Arr(tiles));
    }
}

/// How wide a window is when drawn at a dock tile's height.
///
/// Square for a window with no area yet, which is a window the compositor has
/// not placed — better a tile the right size for nothing than one with no width
/// at all, which GTK would collapse to invisible.
fn tile_width(geometry: Rect, height: u32) -> u32 {
    if geometry.width <= 0 || geometry.height <= 0 {
        return height;
    }
    let ratio = f64::from(geometry.width) / f64::from(geometry.height);
    (f64::from(height) * ratio).round().max(1.0) as u32
}

/// A look arrived, so anything waiting on that one or an older one now knows
/// which window has focus.
fn arrived(waiting: Option<Awaiting>, id: JobId) -> Option<Awaiting> {
    match waiting {
        Some(Awaiting::Look(waited)) if id >= waited => Some(Awaiting::Ready),
        other => other,
    }
}
