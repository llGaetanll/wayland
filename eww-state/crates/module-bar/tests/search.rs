//! The launcher, driven the way a person drives it.
//!
//! Every one of these is a sequence that went wrong at some point while it was
//! being built: the highlight surviving a query that no longer contains it, a
//! Return racing the change event for the last letter typed, and the arrow keys
//! staying grabbed after the panel went away.

mod common;

use common::World;
use module_bar::Event;
use module_search::App;
use module_search::EventSearch;

fn app(name: &str, exec: &str) -> App {
    App {
        name: name.to_string(),
        exec: exec.to_string(),
        icon: String::new(),
        keywords: String::new(),
    }
}

fn installed() -> Vec<App> {
    vec![
        app("Firefox", "firefox"),
        app("Files", "nemo"),
        app("Discord", "discord"),
        app("Disk Usage Analyzer", "baobab"),
    ]
}

fn world() -> World {
    let mut w = World::new();
    w.apps = installed();
    w
}

fn open(w: &mut World) {
    w.feed(&[Event::Search(EventSearch::Toggled)]);
}

fn typed(w: &mut World, text: &str) {
    w.feed(&[Event::Search(EventSearch::Queried {
        text: text.to_string(),
    })]);
}

/// The names shown, in the order they are shown.
fn listed(w: &World) -> Vec<String> {
    w.var("search_hits")
        .split("\"name\":\"")
        .skip(1)
        .filter_map(|s| s.split('"').next().map(str::to_string))
        .collect()
}

#[test]
fn opening_indexes_and_shows_a_bare_field() {
    let mut w = world();
    open(&mut w);

    assert!(w.windows().contains(&"search".to_string()));
    assert!(w.windows().contains(&"search-backdrop".to_string()));
    assert_eq!(w.counts().get("index-apps"), Some(&1));
    assert!(listed(&w).is_empty(), "an empty field lists nothing");
    assert_eq!(
        w.var("search_h"),
        "0",
        "and the panel collapses to the field"
    );
    assert!(w.submap, "the arrow keys are ours while the panel is up");
}

#[test]
fn typing_ranks_and_return_launches_the_highlight() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "fi");
    assert_eq!(
        listed(&w),
        ["Files", "Firefox"],
        "shorter name breaks the tie"
    );

    w.feed(&[Event::Search(EventSearch::Moved { delta: 1 })]);
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "fi".to_string(),
    })]);

    assert!(w.log.contains(&"launch firefox".to_string()));
    assert!(
        !w.windows().contains(&"search".to_string()),
        "and it closes"
    );
    assert!(!w.submap, "the arrow keys go back to the desktop");
}

/// eww runs every widget handler on a thread of its own, so the change for the
/// last letter typed and the Return that follows it are two commands racing.
/// Return carries the field for exactly this reason: it is the authority on
/// what was in it, whatever order the two arrive in.
#[test]
fn return_launches_what_was_typed_even_when_it_overtakes_the_last_keystroke() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "disc");
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "disco".to_string(),
    })]);

    assert!(w.log.contains(&"launch discord".to_string()));
}

/// The highlight is a position in a ranking, so a new ranking has to reset it.
/// Left alone, arrowing down to the third result and then typing one more
/// letter launched whatever had moved into third place.
#[test]
fn a_new_query_puts_the_highlight_back_on_top() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "di");
    w.feed(&[Event::Search(EventSearch::Moved { delta: 1 })]);
    typed(&mut w, "dis");
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "dis".to_string(),
    })]);

    assert!(w.log.contains(&"launch discord".to_string()));
}

#[test]
fn the_highlight_stops_at_both_ends() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "fi");
    w.feed(&[Event::Search(EventSearch::Moved { delta: -1 })]);
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "fi".to_string(),
    })]);
    assert!(
        w.log.contains(&"launch nemo".to_string()),
        "up from the top stays"
    );

    let mut w = world();
    open(&mut w);
    typed(&mut w, "fi");
    for _ in 0..5 {
        w.feed(&[Event::Search(EventSearch::Moved { delta: 1 })]);
    }
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "fi".to_string(),
    })]);
    assert!(
        w.log.contains(&"launch firefox".to_string()),
        "and down stays"
    );
}

#[test]
fn a_query_matching_nothing_says_so_rather_than_stopping() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "zzz");

    assert_eq!(w.var("search_none"), "true");
    assert_eq!(w.var("search_h"), "0");
    assert!(w.windows().contains(&"search".to_string()), "and stays up");
}

/// Return on a field that matches nothing must still put the panel away: there
/// is nothing to launch, and leaving it up reads as a swallowed keypress.
#[test]
fn return_on_no_results_closes_without_launching() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "zzz");
    w.feed(&[Event::Search(EventSearch::Accepted {
        text: "zzz".to_string(),
    })]);

    assert!(!w.windows().contains(&"search".to_string()));
    assert!(!w.log.iter().any(|l| l.starts_with("launch ")));
    assert!(!w.submap);
}

#[test]
fn the_keybind_is_a_toggle_and_a_second_open_reindexes() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "fi");
    open(&mut w);
    assert!(!w.windows().contains(&"search".to_string()));
    assert!(!w.submap);

    // Apps come and go while a session is up, so every open rescans.
    w.apps.push(app("Zed", "zed"));
    open(&mut w);
    assert_eq!(w.counts().get("index-apps"), Some(&2));
    assert!(listed(&w).is_empty(), "and the field opens empty");
    typed(&mut w, "zed");
    assert_eq!(listed(&w), ["Zed"]);
}

/// A click reaches us as the command rather than a position, because the list
/// can be rebuilt between the frame the user saw and the click landing on it.
#[test]
fn clicking_a_result_launches_it_and_closes() {
    let mut w = world();
    open(&mut w);
    typed(&mut w, "fi");
    w.feed(&[Event::Search(EventSearch::Picked {
        exec: "firefox".to_string(),
    })]);

    assert!(w.log.contains(&"launch firefox".to_string()));
    assert!(!w.windows().contains(&"search".to_string()));
}

/// The launcher is a keybind popup, like the screenshot preview: it must
/// neither dismiss an open bar dropdown nor be dismissed by one.
#[test]
fn the_launcher_and_the_bar_menus_ignore_each_other() {
    let mut w = world();
    w.menu(module_bar::MenuId::Wifi);
    open(&mut w);
    assert_eq!(w.menus_open(), ["wifi-menu"], "the bar menu is untouched");

    w.feed(&[Event::Search(EventSearch::Dismissed)]);
    assert!(!w.windows().contains(&"search".to_string()));
    assert_eq!(w.menus_open(), ["wifi-menu"]);
}
