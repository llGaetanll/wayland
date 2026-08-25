//! The dock's contents and order, from `dock.conf`.

use eww_state_core::DockApp;
use eww_state_core::EventProbe;

use crate::paths::Paths;

pub fn sample(paths: &Paths) -> EventProbe {
    EventProbe::Dock { apps: read(paths) }
}

/// `dock.conf` is `id<TAB>command<TAB>icon`, one app per line, in dock order.
/// The file *is* the order — there is no second list to disagree with it.
pub fn read(paths: &Paths) -> Vec<DockApp> {
    parse(&std::fs::read_to_string(&paths.dock_conf).unwrap_or_default())
}

pub fn write(paths: &Paths, apps: &[DockApp]) -> std::io::Result<()> {
    let mut text = String::from("# id\tcommand\ticon — order here is dock order\n");
    for a in apps {
        text.push_str(&format!("{}\t{}\t{}\n", a.id, a.cmd, a.icon));
    }
    // Write-then-rename: a crash mid-write must not leave a truncated dock.
    let tmp = paths.dock_conf.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &paths.dock_conf)
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
