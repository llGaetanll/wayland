//! Reading and writing `dock.conf`.

use std::path::Path;

use api::util::class_matches;

use crate::event::EventDock;
use crate::model::DockApp;

pub fn sample(conf: &Path) -> EventDock {
    EventDock::Observed { apps: read(conf) }
}

/// Which dock apps the compositor has a window for. macOS shows the dot for an
/// app that is *running*, and a window is the only evidence of that we can get
/// without a process table walk that would guess at least as often.
pub fn sample_running(ids: &[String]) -> EventDock {
    let out = exec::out("hyprctl", &["clients"]).unwrap_or_default();
    EventDock::Running {
        ids: running_ids(ids, &out),
    }
}

fn running_ids(ids: &[String], clients: &str) -> Vec<String> {
    let classes: Vec<String> = clients
        .lines()
        .filter_map(|l| l.trim().strip_prefix("class:"))
        .map(|c| c.trim().to_ascii_lowercase())
        .filter(|c| !c.is_empty())
        .collect();
    ids.iter()
        .filter(|id| classes.iter().any(|c| class_matches(id, c)))
        .cloned()
        .collect()
}

/// `dock.conf` is `id<TAB>command<TAB>icon`, one app per line, in dock order.
/// The file *is* the order — there is no second list to disagree with it.
pub fn read(conf: &Path) -> Vec<DockApp> {
    parse(&std::fs::read_to_string(conf).unwrap_or_default())
}

pub fn write(conf: &Path, apps: &[DockApp]) -> std::io::Result<()> {
    let mut text = String::from("# id\tcommand\ticon — order here is dock order\n");
    for a in apps {
        text.push_str(&format!("{}\t{}\t{}\n", a.id, a.cmd, a.icon));
    }
    // Write-then-rename: a crash mid-write must not leave a truncated dock.
    let tmp = conf.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, conf)
}

fn parse(text: &str) -> Vec<DockApp> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut f = l.split('\t').map(str::trim).filter(|s| !s.is_empty());
            Some(DockApp {
                id: f.next()?.to_string(),
                cmd: f.next()?.to_string(),
                icon: f.next()?.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::running_ids;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    const CLIENTS: &str = "\
Window 558156ab0980 -> 1:system:
\tworkspace: 3 (3)
\tclass: Alacritty
\tfullscreen: 1

Window 5581569f1230 -> Firefox:
\tworkspace: 1 (1)
\tclass: firefox
\tfullscreen: 0
";

    #[test]
    fn a_window_class_finds_its_dock_app() {
        let found = running_ids(&ids(&["firefox", "alacritty", "discord"]), CLIENTS);
        assert_eq!(found, vec!["firefox", "alacritty"], "discord has no window");
    }

    #[test]
    fn no_windows_is_nothing_running() {
        assert!(running_ids(&ids(&["firefox"]), "").is_empty());
    }

    #[test]
    fn conf_roundtrip() {
        let text =
            "# comment\n\nfirefox\tfirefox\t/i/ff.png\nterm\talacritty\t/i/term.svg\nbroken\n";
        let apps = parse(text);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].id, "firefox");
        assert_eq!(apps[1].cmd, "alacritty");
    }
}
