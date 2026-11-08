//! The screenshot state machine. No IO.

use api::jobs::JobId;
use api::jobs::Jobs;
use api::millis::Millis;
use api::module::Module;
use api::wire_vars::WireVars;

use crate::action::ActionShot;
use crate::config::ConfigShot;
use crate::event::EventShot;
use crate::model::Shot;
use crate::model::ShotAction;

#[derive(Clone, Debug, Default)]
pub struct StateShot {
    cfg: ConfigShot,
    jobs: Jobs,
    staged: Option<Shot>,
    /// A capture is running: the region selector is on screen. A second Print
    /// while it is up is dropped, which is what the lock file used to be for.
    capturing: Option<Inflight>,
    now: Millis,

    /// Print was pressed. Whether that turns into a capture is decided once the
    /// batch is over, so mashing the key is one selector rather than a stack.
    wanted: bool,
    /// Staged files nobody owns any more, waiting to be reaped.
    orphans: Vec<String>,
    /// What the user pressed on the preview, and the file it was about.
    acted: Option<(ShotAction, String)>,
}

#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

impl StateShot {
    pub fn new(cfg: ConfigShot) -> StateShot {
        StateShot {
            cfg,
            ..StateShot::default()
        }
    }

    /// A preview is on screen.
    pub fn is_open(&self) -> bool {
        self.staged.is_some()
    }

    /// A selector is up, so Print does nothing.
    pub fn capturing(&self) -> bool {
        self.capturing.is_some()
    }

    /// Preview box, fitted to the capture's aspect ratio and capped in width so
    /// a wide selection doesn't produce a panel wider than the screen.
    fn preview(&self) -> (u32, u32) {
        let Some(s) = &self.staged else {
            return (0, 0);
        };
        if s.width <= self.cfg.max_width || s.width == 0 {
            (s.width, s.height)
        } else {
            let w = self.cfg.max_width;
            (w, (s.height as u64 * w as u64 / s.width as u64) as u32)
        }
    }
}

impl Module for StateShot {
    type Event = EventShot;
    type Action = ActionShot;
    type Req = ();

    fn handle_event(&mut self, event: &EventShot) {
        match event {
            EventShot::Requested => self.wanted = true,
            EventShot::Finished { id } => {
                if self.capturing.is_some_and(|c| c.id == *id) {
                    self.capturing = None;
                }
            }
            // A selector the user walked away from would otherwise block every
            // later Print press, since only one capture may be in flight.
            EventShot::Tick { now } => {
                self.now = *now;
                if self
                    .capturing
                    .is_some_and(|c| now.since(c.since) >= self.cfg.timeout)
                {
                    self.capturing = None;
                }
            }
            EventShot::Staged { shot } => {
                // A second capture while a preview is up replaces it; the old
                // staged file is ours to clean up.
                if let Some(prev) = self.staged.replace(shot.clone()) {
                    if prev.path != shot.path {
                        self.orphans.push(prev.path);
                    }
                }
            }
            EventShot::Acted { action } => {
                let Some(shot) = self.staged.take() else {
                    return;
                };
                self.acted = Some((*action, shot.path));
            }
        }
    }

    fn emit_actions(&mut self, actions: &mut Vec<ActionShot>) -> Option<()> {
        if std::mem::take(&mut self.wanted) && self.capturing.is_none() {
            let id = self.jobs.take();
            actions.push(ActionShot::Capture { id });
            self.capturing = Some(Inflight {
                id,
                since: self.now,
            });
        }
        for path in std::mem::take(&mut self.orphans) {
            actions.push(ActionShot::Delete { path });
        }
        if let Some((action, path)) = self.acted.take() {
            match action {
                ShotAction::Copy => actions.push(ActionShot::Copy { path }),
                ShotAction::Save => actions.push(ActionShot::Save { path }),
                ShotAction::Discard => actions.push(ActionShot::Delete { path }),
                // Click-outside leaves the file staged: the next capture reaps
                // it, and nothing is lost if the dismissal was a slip.
                ShotAction::Dismiss => {}
            }
        }
        None
    }

    fn vars(&self, out: &mut WireVars) {
        let (w, h) = self.preview();
        out.set(
            "shot_path",
            self.staged.as_ref().map(|s| s.path.as_str()).unwrap_or(""),
        );
        out.set("shot_w", w);
        out.set("shot_h", h);
    }
}
