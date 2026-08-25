//! Making eww match the state machine.
//!
//! The machine says what *should* be true; this decides the smallest set of eww
//! commands that makes it so. It is the only thing in the process that talks to
//! eww, which is what keeps the published vars and the open windows from
//! drifting apart the way they did when every script pushed its own.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use api::WireVars;
use module_bar::Window;

/// The eww commands we need. Abstracted so the reconciliation logic can be
/// tested without a compositor.
pub trait EwwSink {
    /// `eww update k=v …`
    fn update(&mut self, pairs: &[(&str, &str)]) -> bool;
    /// `eww open-many …`
    fn open(&mut self, windows: &[String]) -> bool;
    /// `eww close …`
    fn close(&mut self, windows: &[String]) -> bool;
    /// `eww close-all`
    fn close_all(&mut self) -> bool;
}

pub struct Reconciler {
    published: BTreeMap<String, String>,
    open: BTreeSet<Window>,
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl Reconciler {
    pub fn new() -> Self {
        Reconciler {
            published: BTreeMap::new(),
            open: BTreeSet::new(),
        }
    }

    /// Forget what we believe eww knows. Called when eww restarts: its vars are
    /// back at their defaults and every window is closed, so the next sync must
    /// send everything rather than a diff against a world that no longer exists.
    pub fn forget(&mut self) {
        self.published.clear();
        self.open.clear();
    }

    /// Start from a known-empty screen. Anything a previous daemon left behind
    /// is closed here, so no orphaned surface can survive a restart.
    pub fn reset(&mut self, sink: &mut impl EwwSink) {
        sink.close_all();
        self.forget();
    }

    pub fn sync_vars(&mut self, vars: &WireVars, sink: &mut impl EwwSink) {
        let changed: Vec<(&str, &str)> = vars
            .iter()
            .filter(|(k, v)| self.published.get(*k).map(String::as_str) != Some(*v))
            .collect();
        if changed.is_empty() {
            return;
        }
        if sink.update(&changed) {
            for (k, v) in changed {
                self.published.insert(k.to_string(), v.to_string());
            }
        }
    }

    /// Close what should not be up, then open what should — in that order, so a
    /// menu swap never has two panels on screen at the same moment.
    pub fn sync_windows(&mut self, want: &BTreeSet<Window>, sink: &mut impl EwwSink) {
        let to_close: Vec<Window> = self.open.difference(want).copied().collect();
        let to_open: Vec<Window> = want.difference(&self.open).copied().collect();

        if !to_close.is_empty() {
            let names: Vec<String> = to_close.iter().map(|w| w.name()).collect();
            // Assume closed either way: a close that failed because the window
            // was already gone must not leave us thinking it is still up.
            sink.close(&names);
            for w in to_close {
                self.open.remove(&w);
            }
        }
        if !to_open.is_empty() {
            let names: Vec<String> = to_open.iter().map(|w| w.name()).collect();
            // Only record an open that worked. eww does not deduplicate opens,
            // so believing in a window we never opened would orphan the surface
            // the next time we "reopen" it.
            if sink.open(&names) {
                self.open.extend(to_open);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use module_bar::MenuId;

    #[derive(Default)]
    struct Fake {
        calls: Vec<String>,
        fail_open: bool,
    }

    impl EwwSink for Fake {
        fn update(&mut self, pairs: &[(&str, &str)]) -> bool {
            let body: Vec<String> = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect();
            self.calls.push(format!("update {}", body.join(" ")));
            true
        }
        fn open(&mut self, windows: &[String]) -> bool {
            self.calls.push(format!("open {}", windows.join(" ")));
            !self.fail_open
        }
        fn close(&mut self, windows: &[String]) -> bool {
            self.calls.push(format!("close {}", windows.join(" ")));
            true
        }
        fn close_all(&mut self) -> bool {
            self.calls.push("close-all".into());
            true
        }
    }

    fn vars(pairs: &[(&'static str, &str)]) -> WireVars {
        let mut v = WireVars::default();
        for (k, val) in pairs {
            v.set(k, val);
        }
        v
    }

    #[test]
    fn only_changed_vars_are_sent() {
        let (mut r, mut f) = (Reconciler::new(), Fake::default());
        r.sync_vars(&vars(&[("clock", "10:00"), ("theme", "light")]), &mut f);
        r.sync_vars(&vars(&[("clock", "10:00"), ("theme", "light")]), &mut f);
        r.sync_vars(&vars(&[("clock", "10:01"), ("theme", "light")]), &mut f);
        assert_eq!(
            f.calls,
            ["update clock=10:00 theme=light", "update clock=10:01"]
        );
    }

    #[test]
    fn windows_close_before_they_open() {
        let (mut r, mut f) = (Reconciler::new(), Fake::default());
        let mut want = BTreeSet::from([Window::Bar, Window::Dock]);
        r.sync_windows(&want, &mut f);

        want.insert(Window::Menu(MenuId::Wifi));
        want.insert(Window::MenuBackdrop);
        r.sync_windows(&want, &mut f);

        // Switching menus: the old panel goes before the new one arrives, and
        // the shared backdrop is left alone rather than flickering.
        want.remove(&Window::Menu(MenuId::Wifi));
        want.insert(Window::Menu(MenuId::Bt));
        r.sync_windows(&want, &mut f);

        assert_eq!(
            f.calls,
            [
                "open bar dock",
                "open wifi-menu menu-backdrop",
                "close wifi-menu",
                "open bt-menu",
            ]
        );
    }

    #[test]
    fn nothing_is_said_when_nothing_changed() {
        let (mut r, mut f) = (Reconciler::new(), Fake::default());
        let want = BTreeSet::from([Window::Bar]);
        r.sync_windows(&want, &mut f);
        r.sync_windows(&want, &mut f);
        assert_eq!(f.calls, ["open bar"]);
    }

    #[test]
    fn a_failed_open_is_retried_rather_than_assumed() {
        let mut r = Reconciler::new();
        let mut f = Fake {
            fail_open: true,
            ..Default::default()
        };
        let want = BTreeSet::from([Window::Bar]);
        r.sync_windows(&want, &mut f);
        r.sync_windows(&want, &mut f);
        assert_eq!(f.calls, ["open bar", "open bar"]);
    }

    #[test]
    fn after_an_eww_restart_everything_is_republished() {
        let (mut r, mut f) = (Reconciler::new(), Fake::default());
        r.sync_vars(&vars(&[("clock", "10:00")]), &mut f);
        r.sync_windows(&BTreeSet::from([Window::Bar]), &mut f);
        f.calls.clear();

        r.reset(&mut f);
        r.sync_vars(&vars(&[("clock", "10:00")]), &mut f);
        r.sync_windows(&BTreeSet::from([Window::Bar]), &mut f);
        assert_eq!(f.calls, ["close-all", "update clock=10:00", "open bar"]);
    }
}
