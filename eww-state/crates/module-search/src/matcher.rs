//! Ranking apps against what has been typed.
//!
//! One score per app, best first. There are no buckets and no separate "fuzzy
//! pass": an exact hit and a scattered one are measured by the same function,
//! and the reason an exact hit wins is that it is worth more, not that it was
//! looked for first.
//!
//! What the numbers are built to guarantee, in order: a name containing the
//! query unbroken beats one where the letters are merely present in order; a
//! match at the start of the name beats one in the middle; a match on the name
//! beats one on something the user cannot see, like a keyword or the command.
//! The subsequence score is bounded well under [`EXACT`] so no amount of
//! scattered-letter luck can overtake a real substring.

use crate::model::App;

/// A name that contains the query unbroken. Far above anything the
/// letters-in-order scorer can reach, which is what makes "exact wins" a
/// property of the arithmetic rather than a rule applied on top of it.
const EXACT: i32 = 1_000;
/// ...and it starts there.
const PREFIX: i32 = 400;
/// ...or it starts a word inside it: "office" in "LibreOffice Calc".
const WORD: i32 = 200;

/// A letter that opens a word. What people type is almost always initials, so
/// this is the single most useful signal the scattered scorer has.
const OPENS_WORD: i32 = 12;
/// A letter directly after the previous one. Enough of these in a row is what
/// makes a near-exact run outrank a scattered one.
const RUNS_ON: i32 = 8;
/// Any other letter. Present, in order, and that is all.
const PRESENT: i32 = 2;

/// Matched somewhere the user cannot see. Ranked below every visible match of
/// the same quality, but still above a poor one: typing "browser" should find
/// Firefox, just never ahead of an app actually called Browser.
const HIDDEN: i32 = -600;
/// What a scattered match has to be worth to be shown at all.
///
/// Without this the launcher is unusable: for a two-letter query almost every
/// app has those letters somewhere in order, so "fr" listed LibreOffice Draw
/// and Impress under Firefox. A match whose letters each open a word or run on
/// from the last scores comfortably positive; one that only reaches the letters
/// by skipping half the name does not. Zero is therefore the natural line, and
/// it falls out of the bonuses rather than being tuned to this machine's apps.
///
/// It also means scattered letters only ever match a *name*: a hidden field
/// starts [`HIDDEN`] below and cannot climb back over the line, so a keyword
/// has to contain the query outright to count. That is the behaviour we want —
/// letters scattered through text the user cannot see are not evidence of
/// anything.
const FLOOR: i32 = 1;

/// The apps matching `query`, best first, capped at `limit`.
///
/// An empty query matches nothing: an empty launcher shows a bare field the
/// way Spotlight does, rather than an arbitrary alphabetical wall of apps.
pub fn rank<'a>(apps: &'a [App], query: &str, limit: usize) -> Vec<&'a App> {
    let q: Vec<char> = query.trim().to_lowercase().chars().collect();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits: Vec<(i32, usize, &App)> = apps
        .iter()
        .filter_map(|a| score(a, &q))
        .filter(|&(s, ..)| s >= FLOOR)
        .collect();
    // Score, then the shorter name: for "fi", `Files` is a likelier answer than
    // `Firefox Nightly Profile Manager`. The name itself is the last tie-break,
    // so the order never depends on how the directories happened to be read.
    hits.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.name.cmp(&b.2.name))
    });
    hits.into_iter().take(limit).map(|(_, _, a)| a).collect()
}

/// The best case this app can make for itself, or `None` if it cannot.
///
/// The command is not searched here: it is folded into `keywords` unwrapped,
/// because the runnable form of a terminal app begins with the terminal and
/// matching that put every one of them under "alac".
fn score<'a>(app: &'a App, q: &[char]) -> Option<(i32, usize, &'a App)> {
    let on = |text: &str, penalty: i32| {
        let hay: Vec<char> = text.to_lowercase().chars().collect();
        match_score(&hay, q).map(|s| s + penalty)
    };
    let best = [on(&app.name, 0), on(&app.keywords, HIDDEN)]
        .into_iter()
        .flatten()
        .max()?;
    Some((best, app.name.chars().count(), app))
}

/// How well one piece of text answers the query.
fn match_score(hay: &[char], q: &[char]) -> Option<i32> {
    if let Some(at) = find(hay, q) {
        let placed = if at == 0 {
            PREFIX
        } else if opens_word(hay, at) {
            WORD
        } else {
            0
        };
        // Earlier is better, but only up to a point: past a few dozen
        // characters the difference stops meaning anything.
        return Some(EXACT + placed - (at.min(40) as i32));
    }
    subsequence(hay, q)
}

/// Where `needle` appears in `hay`, unbroken.
fn find(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// Every letter present, in order, but not together.
///
/// Taken greedily from the left, which is not always the arrangement scoring
/// highest — but the queries this serves are two or three letters long, where
/// the greedy run and the best run are the same one often enough that a full
/// search would cost more than it settles.
fn subsequence(hay: &[char], q: &[char]) -> Option<i32> {
    let mut score = 0;
    let mut from = 0;
    let mut previous: Option<usize> = None;
    for &wanted in q {
        let at = (from..hay.len()).find(|&i| hay[i] == wanted)?;
        score += if opens_word(hay, at) {
            OPENS_WORD
        } else if previous.is_some_and(|p| p + 1 == at) {
            RUNS_ON
        } else {
            PRESENT
        };
        // Letters the user had to skip past are letters they did not type.
        score -= ((at - from) as i32).min(4);
        previous = Some(at);
        from = at + 1;
    }
    Some(score)
}

fn opens_word(hay: &[char], at: usize) -> bool {
    at == 0 || !hay[at - 1].is_alphanumeric()
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
        assert_eq!(names(&apps, "disc")[0], "Discord");
    }

    /// The point of the whole rewrite: letters that are merely present, in
    /// order, still find the app.
    #[test]
    fn scattered_letters_find_it() {
        let apps = vec![app("Firefox", ""), app("Files", "")];
        assert_eq!(names(&apps, "fr"), ["Firefox"]);
        assert_eq!(names(&apps, "ffx"), ["Firefox"]);
    }

    /// And never at the cost of the obvious answer. An app whose name contains
    /// the query unbroken outranks one that merely has the letters, however
    /// flattering the arrangement.
    #[test]
    fn an_exact_run_always_beats_a_scattered_one() {
        let apps = vec![
            app("Fluent Reader Extra Edition", ""),
            app("Frobnicator", ""),
        ];
        assert_eq!(
            names(&apps, "fr"),
            ["Frobnicator", "Fluent Reader Extra Edition"],
            "a substring wins even against every word-start bonus going"
        );
    }

    #[test]
    fn a_word_in_the_middle_beats_a_letter_in_the_middle() {
        let apps = vec![app("LibreOffice Calc", ""), app("Backup Utility", "")];
        assert_eq!(names(&apps, "calc")[0], "LibreOffice Calc");
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
    fn letters_that_are_not_all_there_match_nothing() {
        let apps = vec![app("Firefox", "")];
        assert!(names(&apps, "zzz").is_empty());
        assert!(names(&apps, "xof").is_empty(), "and order matters");
    }

    /// For a two-letter query almost every app has the letters somewhere in
    /// order. Without the floor, "fr" listed four LibreOffice entries under
    /// Firefox and the panel was useless.
    #[test]
    fn letters_reached_only_by_skipping_are_not_a_match() {
        let apps = vec![app("Firefox", ""), app("LibreOffice Draw", "")];
        assert_eq!(
            names(&apps, "fr"),
            ["Firefox"],
            "the f and r in LibreOffice Draw are not what anyone typed"
        );
    }

    /// Scattered letters are evidence only in text the user can actually read.
    #[test]
    fn a_hidden_field_has_to_contain_the_query_outright() {
        let scattered = vec![app("Notes", "keeps your writing safe")];
        assert!(
            names(&scattered, "kyws").is_empty(),
            "initials across a keyword line mean nothing"
        );
        let outright = vec![app("Files", "nemo")];
        assert_eq!(names(&outright, "nemo"), ["Files"]);
    }

    #[test]
    fn the_cap_is_honoured() {
        let apps: Vec<App> = (0..20).map(|i| app(&format!("App {i}"), "")).collect();
        assert_eq!(rank(&apps, "app", 5).len(), 5);
    }
}
