//! Ranking apps against what has been typed.
//!
//! Not a fuzzy matcher, deliberately. A launcher is judged on whether the app
//! you meant is first after two or three characters, and over a list this size
//! — a few dozen entries — what achieves that is plain containment ordered by
//! where the match landed. A name that starts with the query beats one that
//! has it in the middle, which beats a match nobody can see because it came
//! from a keyword. Subsequence scoring would rank `Disk Usage Analyzer` above
//! `Discord` for "disc" on a tie-break nobody typed.

use crate::model::App;

/// Where the query was found. Lower sorts first, so the order of these arms is
/// the ranking.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    NameStart,
    WordStart,
    NameAnywhere,
    Keyword,
    Command,
}

/// The apps matching `query`, best first, capped at `limit`.
///
/// An empty query matches nothing: an empty launcher shows a bare field the
/// way Spotlight does, rather than an arbitrary alphabetical wall of apps.
pub fn rank<'a>(apps: &'a [App], query: &str, limit: usize) -> Vec<&'a App> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(Tier, usize, &App)> = apps
        .iter()
        .filter_map(|a| tier(a, &q).map(|t| (t, a.name.chars().count(), a)))
        .collect();
    // Shorter names first within a tier: for "fi", `Files` is a likelier answer
    // than `Firefox Nightly Profile Manager`. The name is the last tie-break so
    // the order never depends on how the directories happened to be read.
    hits.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.name.cmp(&b.2.name))
    });
    hits.into_iter().take(limit).map(|(_, _, a)| a).collect()
}

fn tier(app: &App, q: &str) -> Option<Tier> {
    let name = app.name.to_lowercase();
    if name.starts_with(q) {
        return Some(Tier::NameStart);
    }
    if name
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| word.starts_with(q))
    {
        return Some(Tier::WordStart);
    }
    if name.contains(q) {
        return Some(Tier::NameAnywhere);
    }
    if app.keywords.to_lowercase().contains(q) {
        return Some(Tier::Keyword);
    }
    if app.exec.to_lowercase().contains(q) {
        return Some(Tier::Command);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::rank;
    use crate::model::App;

    fn app(name: &str, keywords: &str) -> App {
        App {
            name: name.to_string(),
            exec: name.to_lowercase(),
            icon: String::new(),
            keywords: keywords.to_string(),
        }
    }

    fn names(apps: &[App], q: &str) -> Vec<String> {
        rank(apps, q, 8)
            .into_iter()
            .map(|a| a.name.clone())
            .collect()
    }

    #[test]
    fn a_name_that_starts_with_the_query_comes_first() {
        let apps = vec![
            app("Disk Usage Analyzer", ""),
            app("Discord", ""),
            app("Mission Center", "discover"),
        ];
        assert_eq!(names(&apps, "disc"), ["Discord", "Mission Center"]);
    }

    #[test]
    fn a_word_in_the_middle_still_matches() {
        let apps = vec![app("GNU Image Manipulation Program", "")];
        assert_eq!(names(&apps, "image").len(), 1);
    }

    #[test]
    fn keywords_are_searched_but_rank_below_names() {
        let apps = vec![app("Firefox", "browser web"), app("Browser Chooser", "")];
        assert_eq!(names(&apps, "browser"), ["Browser Chooser", "Firefox"]);
    }

    #[test]
    fn an_empty_query_matches_nothing() {
        let apps = vec![app("Firefox", "")];
        assert!(names(&apps, "").is_empty(), "an empty field lists nothing");
        assert!(
            names(&apps, "   ").is_empty(),
            "nor does one holding spaces"
        );
    }

    #[test]
    fn the_cap_is_honoured() {
        let apps: Vec<App> = (0..20).map(|i| app(&format!("App {i}"), "")).collect();
        assert_eq!(rank(&apps, "app", 5).len(), 5);
    }
}
