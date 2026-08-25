//! The screenshot preview: capture, then Copy / Save / Discard.
//!
//! Deliberately independent of the bar dropdowns. A Print press must not
//! dismiss an open bar menu, and clicking outside the preview must not dismiss
//! one either, so this machine owns its own window and its own backdrop.

use crate::action::Action;
use crate::action_file::ActionFile;
use crate::action_shell::ActionShell;
use crate::config::Config;
use crate::jobs::Jobs;
use crate::millis::Millis;
use crate::model::Shot;
use crate::model::ShotAction;
use crate::probe::JobId;
use crate::req::Req;
use crate::state::State;
use crate::wire_vars::WireVars;

#[derive(Clone, Debug, Default)]
pub struct StateShot {
    staged: Option<Shot>,
    /// A capture is running: the region selector is on screen. A second Print
    /// while it is up is dropped, which is what the lock file used to be for.
    capturing: Option<Inflight>,
    /// Print was pressed. Whether that turns into a capture is decided once the
    /// batch is over, so mashing the key is one selector rather than a stack.
    wanted: bool,
    /// Staged files nobody owns any more, waiting to be reaped.
    orphans: Vec<String>,
    /// What the user pressed on the preview, and the file it was about.
    acted: Option<(ShotAction, String)>,
    now: Millis,
}

/// The capture we are waiting on, and the tick we started waiting at.
#[derive(Clone, Copy, Debug)]
struct Inflight {
    id: JobId,
    since: Millis,
}

#[derive(Debug)]
pub enum EventShot {
    Capture,
    /// A capture finished and left this file behind.
    Staged {
        shot: Shot,
    },
    Act {
        action: ShotAction,
    },
    /// The clock moved; give up on a selection nobody is making any more.
    Tick {
        now: Millis,
    },
    Finished {
        id: JobId,
    },
}

impl StateShot {
    pub fn is_open(&self) -> bool {
        self.staged.is_some()
    }

    /// Preview box, fitted to the capture's aspect ratio and capped in width so
    /// a wide selection doesn't produce a panel wider than the screen.
    fn preview(&self, cfg: &Config) -> (u32, u32) {
        let Some(s) = &self.staged else {
            return (0, 0);
        };
        if s.width <= cfg.shot_max_width || s.width == 0 {
            (s.width, s.height)
        } else {
            let w = cfg.shot_max_width;
            (w, (s.height as u64 * w as u64 / s.width as u64) as u32)
        }
    }
}

impl State for StateShot {
    type Event = EventShot;

    fn handle_event(&mut self, cfg: &Config, event: EventShot) {
        match event {
            EventShot::Capture => self.wanted = true,
            EventShot::Finished { id } => {
                if self.capturing.is_some_and(|c| c.id == id) {
                    self.capturing = None;
                }
            }
            // A selector the user walked away from would otherwise block every
            // later Print press, since only one capture may be in flight.
            EventShot::Tick { now } => {
                self.now = now;
                if self
                    .capturing
                    .is_some_and(|c| now.since(c.since) >= cfg.capture_timeout)
                {
                    self.capturing = None;
                }
            }
            EventShot::Staged { shot } => {
                // A second capture while a preview is up replaces it; the old
                // staged file is ours to clean up.
                if let Some(prev) = self.staged.replace(shot) {
                    let keep = self.staged.as_ref().is_some_and(|s| s.path == prev.path);
                    if !keep {
                        self.orphans.push(prev.path);
                    }
                }
            }
            EventShot::Act { action } => {
                let Some(shot) = self.staged.take() else {
                    return;
                };
                self.acted = Some((action, shot.path));
            }
        }
    }

    fn emit_actions(
        &mut self,
        _cfg: &Config,
        jobs: &mut Jobs,
        actions: &mut Vec<Action>,
    ) -> Option<Req> {
        if std::mem::take(&mut self.wanted) && self.capturing.is_none() {
            let id = jobs.take();
            actions.push(Action::shell(id, ActionShell::Capture));
            self.capturing = Some(Inflight {
                id,
                since: self.now,
            });
        }
        for path in std::mem::take(&mut self.orphans) {
            actions.push(Action::file(jobs.take(), ActionFile::DeleteShot { path }));
        }
        if let Some((action, path)) = self.acted.take() {
            match action {
                ShotAction::Copy => {
                    actions.push(Action::shell(jobs.take(), ActionShell::CopyShot { path }));
                }
                ShotAction::Save => {
                    actions.push(Action::file(jobs.take(), ActionFile::SaveShot { path }));
                }
                ShotAction::Discard => {
                    actions.push(Action::file(jobs.take(), ActionFile::DeleteShot { path }));
                }
                // Click-outside leaves the file staged: the next capture reaps
                // it, and nothing is lost if the dismissal was a slip.
                ShotAction::Dismiss => {}
            }
        }
        None
    }

    fn vars(&self, cfg: &Config, out: &mut WireVars) {
        let (w, h) = self.preview(cfg);
        out.set(
            "shot_path",
            self.staged.as_ref().map(|s| s.path.as_str()).unwrap_or(""),
        );
        out.set("shot_w", w);
        out.set("shot_h", h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::test_util::run;

    fn staged(path: &str, width: u32, height: u32) -> EventShot {
        EventShot::Staged {
            shot: Shot {
                path: path.into(),
                width,
                height,
            },
        }
    }

    #[test]
    fn a_second_print_during_selection_is_dropped() {
        let mut s = StateShot::default();
        let actions = run(&mut s, EventShot::Capture);
        assert_eq!(actions.len(), 1);
        assert!(run(&mut s, EventShot::Capture).is_empty());
        run(
            &mut s,
            EventShot::Finished {
                id: actions[0].id(),
            },
        );
        assert_eq!(run(&mut s, EventShot::Capture).len(), 1);
    }

    #[test]
    fn restaging_cleans_up_the_previous_file() {
        let mut s = StateShot::default();
        run(&mut s, staged("/a.png", 100, 50));
        let actions = run(&mut s, staged("/b.png", 100, 50));
        assert!(matches!(
            actions.first(),
            Some(Action::ToFile {
                action: ActionFile::DeleteShot { path },
                ..
            }) if path == "/a.png"
        ));
    }

    #[test]
    fn acting_twice_only_acts_once() {
        let mut s = StateShot::default();
        run(&mut s, staged("/a.png", 100, 50));
        let copy = || EventShot::Act {
            action: ShotAction::Copy,
        };
        assert_eq!(run(&mut s, copy()).len(), 1);
        assert!(run(&mut s, copy()).is_empty());
        assert!(!s.is_open());
    }

    #[test]
    fn preview_keeps_the_aspect_ratio() {
        let cfg = Config::new("/i", "/c");
        let mut s = StateShot::default();
        run(&mut s, staged("/a.png", 1600, 900));
        assert_eq!(s.preview(&cfg), (cfg.shot_max_width, 180));
        run(&mut s, staged("/b.png", 200, 100));
        assert_eq!(
            s.preview(&cfg),
            (200, 100),
            "small captures are not blown up"
        );
    }
}
