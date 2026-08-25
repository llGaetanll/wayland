//! The screenshot preview: capture, then Copy / Save / Discard.
//!
//! Deliberately independent of the bar dropdowns. A Print press must not
//! dismiss an open bar menu, and clicking outside the preview must not dismiss
//! one either, so this widget owns its own window and its own backdrop.

use crate::action::ActionKind;
use crate::config::Config;
use crate::ctx::{Ctx, Vars};
use crate::event::JobId;
use crate::model::{Shot as Staged, ShotAction};
use crate::widgets::Widget;

#[derive(Clone, Debug, Default)]
pub struct Shot {
    staged: Option<Staged>,
    /// A capture is running: the region selector is on screen. A second Print
    /// while it is up is dropped, which is what the lock file used to be for.
    capturing: Option<JobId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ev {
    Capture,
    /// A capture finished and left this file behind, at these pixel dimensions.
    Staged(Staged),
    Act(ShotAction),
    Done {
        id: JobId,
    },
}

impl Shot {
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

impl Widget for Shot {
    type Ev = Ev;

    fn update(&mut self, ev: Ev, cx: &mut Ctx) {
        match ev {
            Ev::Capture => {
                if self.capturing.is_none() {
                    let id = cx.dispatch(ActionKind::ShotCapture);
                    self.capturing = Some(id);
                }
            }
            Ev::Done { id } => {
                if self.capturing == Some(id) {
                    self.capturing = None;
                }
            }
            Ev::Staged(shot) => {
                // A second capture while a preview is up replaces it; the old
                // staged file is ours to clean up.
                if let Some(prev) = self.staged.replace(shot) {
                    let keep = self.staged.as_ref().is_some_and(|s| s.path == prev.path);
                    if !keep {
                        cx.dispatch(ActionKind::ShotDelete { path: prev.path });
                    }
                }
            }
            Ev::Act(action) => {
                let Some(shot) = self.staged.take() else {
                    return;
                };
                match action {
                    ShotAction::Copy => {
                        cx.dispatch(ActionKind::ShotCopy { path: shot.path });
                    }
                    ShotAction::Save => {
                        cx.dispatch(ActionKind::ShotSave { path: shot.path });
                    }
                    ShotAction::Discard => {
                        cx.dispatch(ActionKind::ShotDelete { path: shot.path });
                    }
                    // Click-outside leaves the file staged: the next capture
                    // reaps it, and nothing is lost if the dismissal was a slip.
                    ShotAction::Dismiss => {}
                }
            }
        }
    }

    fn vars(&self, cfg: &Config, out: &mut Vars) {
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
    use crate::test_util::{kinds, run};

    fn staged(path: &str, w: u32, h: u32) -> Staged {
        Staged {
            path: path.into(),
            width: w,
            height: h,
        }
    }

    #[test]
    fn a_second_print_during_selection_is_dropped() {
        let mut s = Shot::default();
        let acts = run(&mut s, Ev::Capture);
        assert_eq!(acts.len(), 1);
        assert!(kinds(run(&mut s, Ev::Capture)).is_empty());
        run(&mut s, Ev::Done { id: acts[0].id });
        assert_eq!(kinds(run(&mut s, Ev::Capture)).len(), 1);
    }

    #[test]
    fn restaging_cleans_up_the_previous_file() {
        let mut s = Shot::default();
        run(&mut s, Ev::Staged(staged("/a.png", 100, 50)));
        let acts = run(&mut s, Ev::Staged(staged("/b.png", 100, 50)));
        assert!(matches!(
            kinds(acts).first(),
            Some(ActionKind::ShotDelete { path }) if path == "/a.png"
        ));
    }

    #[test]
    fn acting_twice_only_acts_once() {
        let mut s = Shot::default();
        run(&mut s, Ev::Staged(staged("/a.png", 100, 50)));
        assert_eq!(kinds(run(&mut s, Ev::Act(ShotAction::Copy))).len(), 1);
        assert!(kinds(run(&mut s, Ev::Act(ShotAction::Copy))).is_empty());
        assert!(!s.is_open());
    }

    #[test]
    fn preview_keeps_the_aspect_ratio() {
        let cfg = Config::new("/i", "/c");
        let mut s = Shot::default();
        run(&mut s, Ev::Staged(staged("/a.png", 1600, 900)));
        assert_eq!(s.preview(&cfg), (cfg.shot_max_width, 180));
        run(&mut s, Ev::Staged(staged("/b.png", 200, 100)));
        assert_eq!(
            s.preview(&cfg),
            (200, 100),
            "small captures are not blown up"
        );
    }
}
