//! Reading the desktop entries, and finding a picture for each one.
//!
//! Two jobs that look separate and are not: an entry names its icon by a
//! stem — `Icon=firefox` — which is only a file once a theme has been searched
//! for it. Both happen here, once, when the panel opens, so that typing costs
//! nothing but arithmetic.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use crate::model::App;
use crate::perform::Places;

/// How deep a theme nests its size and category directories. `hicolor` is
/// `64x64/apps`, WhiteSur is `apps/scalable`; nothing needs more than this, and
/// a cap is what keeps a symlink loop from being an infinite walk.
const MAX_DEPTH: usize = 3;

/// The size the launcher draws at, in px, and so what an SVG is rendered to.
const ICON_PX: u32 = 64;

/// Every application on the machine, as the launcher should list it.
pub fn index(places: &Places, terminal: &str) -> Vec<App> {
    let icons = IconIndex::build(&places.icon_dirs);
    let mut seen = BTreeSet::new();
    let mut apps = Vec::new();
    for dir in &places.app_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            // The file name is the entry's id, and the directories are in
            // precedence order: a user's own copy of `firefox.desktop` hides
            // the packaged one rather than appearing beside it.
            let Some(id) = path.file_name().map(|n| n.to_owned()) else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(app) = parse(&text, terminal, &icons, places) {
                apps.push(app);
            }
        }
    }
    apps
}

/// One `.desktop` file, or `None` if it is not something to launch.
fn parse(text: &str, terminal: &str, icons: &IconIndex, places: &Places) -> Option<App> {
    let fields = desktop_entry(text);
    let get = |k: &str| fields.get(k).map(String::as_str).unwrap_or_default();
    if get("Type") != "Application" || get("NoDisplay") == "true" || get("Hidden") == "true" {
        return None;
    }
    let name = get("Name");
    let command = strip_field_codes(get("Exec"));
    if name.is_empty() || command.is_empty() {
        return None;
    }
    // Searched on the bare command, launched with the terminal in front of it.
    // Matching the runnable form would have put every terminal app in the
    // results for "alac", since that is what all their command lines start with.
    let exec = if get("Terminal") == "true" {
        format!("{terminal} {command}")
    } else {
        command.clone()
    };
    Some(App {
        name: name.to_string(),
        exec,
        icon: icons.resolve(get("Icon"), places),
        keywords: format!("{} {} {}", get("GenericName"), get("Keywords"), command),
    })
}

/// The `[Desktop Entry]` group's unlocalized keys.
///
/// Locale variants (`Name[de]`) are skipped rather than collected: the panel
/// draws one language, and a `Name[ca]` overwriting `Name` would be whichever
/// of the forty came last in the file.
fn desktop_entry(text: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(group) = line.strip_prefix('[') {
            inside = group.starts_with("Desktop Entry]");
            continue;
        }
        if !inside || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.contains('[') {
            continue;
        }
        fields
            .entry(key.to_string())
            .or_insert_with(|| value.trim().to_string());
    }
    fields
}

/// Drop the `%f`-style placeholders a launcher is supposed to substitute.
///
/// Nothing is opened *with* an app here — the panel launches it empty — so
/// every one of them expands to nothing. Left in, `%U` reaches the shell as a
/// literal argument and Firefox opens a tab for a file called `%U`.
fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // `%%` is an escaped percent sign, and the only one that survives.
            Some('%') => out.push('%'),
            Some(_) => {}
            None => break,
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Stem to icon file, built once per scan.
struct IconIndex(BTreeMap<String, PathBuf>);

impl IconIndex {
    /// Walk each theme in turn. The first theme holding a stem wins outright,
    /// so the desktop's own icon set is what the launcher draws even where a
    /// fallback theme has a sharper file for the same app.
    fn build(roots: &[PathBuf]) -> IconIndex {
        let mut best: BTreeMap<String, (PathBuf, i32)> = BTreeMap::new();
        for root in roots {
            let mut found: BTreeMap<String, (PathBuf, i32)> = BTreeMap::new();
            walk(root, root, 0, &mut found);
            for (stem, hit) in found {
                best.entry(stem).or_insert(hit);
            }
        }
        IconIndex(best.into_iter().map(|(k, (p, _))| (k, p)).collect())
    }

    /// A PNG eww can draw for this `Icon=` value, or an empty string.
    ///
    /// eww draws through gdk-pixbuf, which on this machine has no SVG loader —
    /// librsvg ships the library but not the loader module — so an SVG is
    /// rendered to a PNG in the cache the first time it is wanted and read from
    /// there ever after.
    fn resolve(&self, icon: &str, places: &Places) -> String {
        if icon.is_empty() {
            return String::new();
        }
        let path = if icon.starts_with('/') {
            PathBuf::from(icon)
        } else {
            match self.0.get(icon) {
                Some(p) => p.clone(),
                None => return String::new(),
            }
        };
        if path.extension().is_some_and(|e| e == "svg") {
            return rasterize(&path, places).unwrap_or_default();
        }
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }
        String::new()
    }
}

/// Collect every icon file under one theme root.
fn walk(root: &Path, dir: &Path, depth: usize, out: &mut BTreeMap<String, (PathBuf, i32)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                walk(root, &path, depth + 1, out);
            }
            continue;
        }
        // Application icons only. A theme's `devices` and `mimetypes` share
        // stems with apps often enough to matter, and `symbolic` is a flat
        // monochrome glyph — right for a 16px bar, wrong for a launcher row.
        let text = path.to_string_lossy();
        let in_apps = text.contains("/apps/") || dir == root;
        if !in_apps || text.contains("/symbolic/") || text.contains("-symbolic.") {
            continue;
        }
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        let score = score(&text);
        if score < 0 {
            continue;
        }
        let better = out.get(&stem).is_none_or(|(_, best)| score > *best);
        if better {
            out.insert(stem, (path, score));
        }
    }
}

/// How well a file suits a launcher row. Negative means it is not an icon.
///
/// A raster file at about the size we draw beats one that has to be scaled, and
/// anything raster beats an SVG, which costs a conversion before it can be
/// drawn at all.
fn score(path: &str) -> i32 {
    let base = match path.rsplit_once('.') {
        Some((_, "png")) => 1_000,
        Some((_, "svg")) => 500,
        Some((_, "xpm")) => 100,
        _ => return -1,
    };
    match size_of(path) {
        Some(px) => base - (px as i32 - ICON_PX as i32).abs(),
        None => base,
    }
}

/// The size a theme directory claims, from `64x64` or a bare `64`.
fn size_of(path: &str) -> Option<u32> {
    path.split('/')
        .filter_map(|part| part.split_once('x').map_or(part, |(w, _)| w).parse().ok())
        .next_back()
}

/// Render an SVG into the cache, and hand back the PNG.
fn rasterize(svg: &Path, places: &Places) -> Option<String> {
    let stem = svg.file_stem()?.to_string_lossy();
    let dir = places.cache.join("search-icons");
    let png = dir.join(format!("{stem}.png"));
    if png.exists() {
        return Some(png.to_string_lossy().into_owned());
    }
    std::fs::create_dir_all(&dir).ok()?;
    let px = ICON_PX.to_string();
    let ok = exec::ok(
        "rsvg-convert",
        &[
            "-w",
            &px,
            "-h",
            &px,
            "-o",
            &png.to_string_lossy(),
            &svg.to_string_lossy(),
        ],
    );
    ok.then(|| png.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::desktop_entry;
    use super::parse;
    use super::size_of;
    use super::strip_field_codes;
    use super::IconIndex;

    #[test]
    fn field_codes_are_dropped_but_escaped_percents_are_not() {
        assert_eq!(strip_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_field_codes("gimp-2.10 %U"), "gimp-2.10");
        assert_eq!(strip_field_codes("env X=100%% mpv"), "env X=100% mpv");
        assert_eq!(
            strip_field_codes("code --unity-launch %F"),
            "code --unity-launch"
        );
    }

    #[test]
    fn only_the_desktop_entry_group_is_read() {
        let text = "\
[Desktop Entry]
Name=Firefox
Exec=firefox %u

[Desktop Action new-window]
Name=New Window
Exec=firefox --new-window
";
        let f = desktop_entry(text);
        assert_eq!(f.get("Name").map(String::as_str), Some("Firefox"));
        assert_eq!(
            f.get("Exec").map(String::as_str),
            Some("firefox %u"),
            "an action's Exec must not overwrite the entry's"
        );
    }

    #[test]
    fn localized_names_are_skipped() {
        let text = "[Desktop Entry]\nName=Web Browser\nName[de]=Webbrowser\n";
        let f = desktop_entry(text);
        assert_eq!(f.get("Name").map(String::as_str), Some("Web Browser"));
        assert!(!f.contains_key("Name[de]"));
    }

    /// A terminal app is launched through the terminal but must not be found
    /// by its name: searching "alac" once listed every terminal app there was.
    #[test]
    fn a_terminal_app_is_searched_by_its_own_command() {
        let text = "[Desktop Entry]\nType=Application\nName=Vim\nExec=vim %F\nTerminal=true\n";
        let icons = IconIndex(std::collections::BTreeMap::new());
        let places = crate::perform::Places {
            app_dirs: Vec::new(),
            icon_dirs: Vec::new(),
            cache: std::path::PathBuf::new(),
        };
        let app = parse(text, "alacritty -e", &icons, &places).expect("an application");
        assert_eq!(app.exec, "alacritty -e vim", "but it is run through one");
        assert!(app.keywords.contains("vim"));
        assert!(
            !app.keywords.contains("alacritty"),
            "the terminal is not something this app is called"
        );
    }

    #[test]
    fn a_theme_directory_says_what_size_it_holds() {
        assert_eq!(
            size_of("/usr/share/icons/hicolor/64x64/apps/a.png"),
            Some(64)
        );
        assert_eq!(size_of("/usr/share/icons/WhiteSur/apps/32/a.svg"), Some(32));
        assert_eq!(size_of("/usr/share/pixmaps/a.png"), None);
    }
}
