//! What the Spaces module does, said as the conversations it can have.
//!
//! Nothing here reaches inside `StateSpaces`. A test plays the compositor:
//! answer the looks with the layout it wants to describe, report the moves as
//! finished, and assert on what came back out.
//!
//! Not written through `eval!`, unlike every other module's tests: each of
//! these conversations turns on the id a look went out with — which look is
//! being answered is the whole subject — and the macro deliberately keeps ids
//! out of a test.

use api::JobId;
use api::Module;
use module_spaces::ActionSpaces;
use module_spaces::Client;
use module_spaces::ConfigSpaces;
use module_spaces::EventSpaces;
use module_spaces::MoveTarget;
use module_spaces::Rect;
use module_spaces::Snapshot;
use module_spaces::StateSpaces;
use module_spaces::WindowAddr;
use module_spaces::WorkspaceId;

fn addr(s: &str) -> WindowAddr {
    WindowAddr::parse(s).expect("a window address")
}

/// One window: where it is, and whether it is filling the screen.
fn client(a: &str, workspace: i32, fullscreen: bool) -> Client {
    Client {
        addr: addr(a),
        workspace: WorkspaceId(workspace),
        workspace_name: workspace.to_string(),
        fullscreen,
        // This desktop floats every window by default; the tests that care
        // about tiling say so themselves.
        floating: true,
        monitor: 0,
        geometry: Rect {
            x: 10,
            y: 20,
            width: 800,
            height: 600,
        },
        class: "firefox".to_string(),
        title: "Mozilla Firefox".to_string(),
    }
}

/// The same window, put away where the compositor does not draw it.
fn hidden(a: &str) -> Client {
    Client {
        workspace: WorkspaceId(-98),
        workspace_name: HIDDEN.to_string(),
        ..client(a, -98, false)
    }
}

/// The workspace this module keeps minimized windows on, spelled the way the
/// compositor reports it.
const HIDDEN: &str = "special:minimized";

/// The id a look went out with, so a test can answer that exact one.
fn look_id(actions: &[ActionSpaces]) -> JobId {
    actions
        .iter()
        .find_map(|a| match a {
            ActionSpaces::Look { id } => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no look was dispatched: {actions:?}"))
}

/// Answer the look in `actions` with what the compositor would have said.
fn answer(
    state: &mut StateSpaces,
    actions: &[ActionSpaces],
    focused: i32,
    clients: Vec<Client>,
) -> Vec<ActionSpaces> {
    let focused = WorkspaceId(focused);
    let snapshot = Snapshot {
        focused,
        // Whatever is on the focused workspace has the focus, which is all a
        // test ever needs to say about it.
        active: clients
            .iter()
            .find(|c| c.workspace == focused)
            .map(|c| c.addr.clone()),
        clients,
    };
    let id = look_id(actions);
    drive(state, EventSpaces::Sampled { id, snapshot })
}

/// Report every move in `actions` as done, the way the runtime does once the
/// compositor has carried it out.
fn moves_done(state: &mut StateSpaces, actions: &[ActionSpaces]) -> Vec<ActionSpaces> {
    let ids: Vec<JobId> = actions
        .iter()
        .filter_map(|a| match a {
            ActionSpaces::Move { id, .. } | ActionSpaces::Minimize { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert!(!ids.is_empty(), "no move was dispatched: {actions:?}");
    let events: Vec<EventSpaces> = ids
        .into_iter()
        .map(|id| EventSpaces::Finished { id })
        .collect();
    let mut out = Vec::new();
    state.handle_events(&events);
    state.emit_actions(&mut out);
    out
}

fn drive(state: &mut StateSpaces, event: EventSpaces) -> Vec<ActionSpaces> {
    api::module::drive(state, event).0
}

/// The one action these events produced.
fn only(actions: &[ActionSpaces]) -> &ActionSpaces {
    assert_eq!(actions.len(), 1, "expected one action, got:\n{actions:#?}");
    &actions[0]
}

/// Nothing had to happen. A real assertion, not an omission.
fn quiet(actions: &[ActionSpaces]) {
    assert!(actions.is_empty(), "expected nothing, got:\n{actions:#?}");
}

/// Drive the module to where one window is in a Space: it went fullscreen on
/// workspace 1, was moved onto workspace 9, and the move settled.
fn in_a_space() -> StateSpaces {
    let mut s = StateSpaces::default();
    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, true)]);
    assert!(
        matches!(
            only(&a),
            ActionSpaces::Move {
                to: MoveTarget::Empty,
                ..
            }
        ),
        "{a:#?}"
    );

    // Settling asks for a look of its own; the world after the move answers it.
    let a = moves_done(&mut s, &a);
    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, true)]);
    quiet(&a);
    assert!(s.in_space(&addr("0xa")), "the window is in a Space");
    s
}

#[test]
fn going_fullscreen_moves_the_window_to_a_space_of_its_own() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::Due);
    assert!(matches!(only(&a), ActionSpaces::Look { .. }), "{a:#?}");

    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    quiet(&a);

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, true)]);
    assert!(
        matches!(
            only(&a),
            ActionSpaces::Move {
                to: MoveTarget::Empty,
                ..
            }
        ),
        "{a:#?}"
    );
    assert!(s.in_space(&addr("0xa")));
}

/// The bug the whole design turns on. Moving a fullscreen window makes the
/// compositor re-emit fullscreen — a transient `0` that reads exactly like the
/// user leaving it. Act on that and the window is sent home, which emits the
/// echo again, forever. The Lua version swallowed these with a 140ms debounce
/// and hoped; here the module knows the move is its own.
#[test]
fn the_echo_from_our_own_move_is_not_the_user_leaving_fullscreen() {
    let mut s = StateSpaces::default();
    let a = drive(&mut s, EventSpaces::Due);
    let moving = answer(&mut s, &a, 1, vec![client("0xa", 1, true)]);
    assert!(
        matches!(only(&moving), ActionSpaces::Move { .. }),
        "{moving:#?}"
    );

    // The echo: not fullscreen any more, says the compositor, while our own
    // move is still in flight. Looking again is fine; believing it is not.
    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, false)]);
    quiet(&a);
    assert!(s.in_space(&addr("0xa")), "the echo did not evict it");

    // The move lands. What settles it is a fresh look, and that one is the
    // first believed.
    let a = moves_done(&mut s, &moving);
    assert!(matches!(only(&a), ActionSpaces::Look { .. }), "{a:#?}");
    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, true)]);
    quiet(&a);
    assert!(s.in_space(&addr("0xa")), "and it stayed where it was put");
}

#[test]
fn leaving_fullscreen_puts_it_back_where_it_came_from() {
    let mut s = in_a_space();

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, false)]);
    assert!(
        matches!(
            only(&a),
            ActionSpaces::Move {
                to: MoveTarget::Workspace(WorkspaceId(1)),
                ..
            }
        ),
        "it goes back to workspace 1:\n{a:#?}"
    );
    assert!(!s.in_space(&addr("0xa")), "it is an ordinary window again");
}

/// The pruning sweep the shell version ran over a directory of files, except
/// that there is nothing left behind to sweep.
#[test]
fn a_window_that_vanishes_leaves_no_origin_behind() {
    let mut s = in_a_space();

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, Vec::new());
    quiet(&a);
    assert!(
        !s.in_space(&addr("0xa")),
        "the window took its origin with it"
    );
}

/// Closing inside a Space has to go home first: a Space emptied while it is
/// still focused lingers instead of being destroyed.
#[test]
fn closing_inside_a_space_goes_home_first() {
    let mut s = in_a_space();

    let a = drive(&mut s, EventSpaces::CloseRequested);
    assert!(matches!(only(&a), ActionSpaces::Look { .. }), "{a:#?}");

    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, true)]);
    assert_eq!(a.len(), 2, "home, then close:\n{a:#?}");
    assert!(
        matches!(
            a[0],
            ActionSpaces::Focus {
                workspace: WorkspaceId(1)
            }
        ),
        "{a:#?}"
    );
    assert!(matches!(a[1], ActionSpaces::Close { .. }), "{a:#?}");
}

#[test]
fn closing_an_ordinary_window_just_closes_it() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::CloseRequested);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    assert!(matches!(only(&a), ActionSpaces::Close { .. }), "{a:#?}");
}

/// A close whose look produced nothing must not wait for one forever: the next
/// look would otherwise close whatever happened to be focused by then.
#[test]
fn a_close_whose_look_failed_is_dropped() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::CloseRequested);
    let a = drive(&mut s, EventSpaces::Finished { id: look_id(&a) });
    quiet(&a);

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    quiet(&a);
}

#[test]
fn two_windows_can_be_in_spaces_at_once() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(
        &mut s,
        &a,
        1,
        vec![client("0xa", 1, true), client("0xb", 1, true)],
    );
    assert_eq!(a.len(), 2, "one move each:\n{a:#?}");
    assert!(s.in_space(&addr("0xa")) && s.in_space(&addr("0xb")));
}

/// What the bar asks this module, and the only thing it asks.
#[test]
fn the_bar_hides_only_while_a_fullscreen_space_is_focused() {
    assert!(
        !StateSpaces::default().fullscreen(),
        "nothing has been seen yet"
    );

    let mut s = in_a_space();
    assert!(s.fullscreen(), "the Space is focused");

    // Swipe away to the workspace it came from, where an ordinary window is.
    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(
        &mut s,
        &a,
        1,
        vec![client("0xa", 9, true), client("0xb", 1, false)],
    );
    quiet(&a);
    assert!(
        !s.fullscreen(),
        "a Space nobody is looking at does not hide the bar"
    );

    let a = drive(&mut s, EventSpaces::Due);
    answer(&mut s, &a, 9, vec![client("0xa", 9, true)]);
    assert!(s.fullscreen(), "and it hides again on the way back");
}

/// Drive the module to where one window is minimized: it was focused on
/// workspace 1, was photographed and hidden, and the move settled.
fn minimized() -> StateSpaces {
    let mut s = StateSpaces::default();
    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    quiet(&a);

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    assert!(
        matches!(
            only(&a),
            ActionSpaces::Minimize {
                to: MoveTarget::Hidden(_),
                ..
            }
        ),
        "{a:#?}"
    );
    let actions = a;

    let a = drive(&mut s, EventSpaces::Captured { addr: addr("0xa") });
    quiet(&a);
    settle_the_move(&mut s, &actions);
    s
}

/// The same, for a capture that failed: grim said nothing, so no `Captured`.
fn minimized_without_a_photograph() -> StateSpaces {
    let mut s = StateSpaces::default();
    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    quiet(&a);

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    settle_the_move(&mut s, &a);
    s
}

/// Report the move in `actions` as done and answer the look that settles it.
fn settle_the_move(state: &mut StateSpaces, actions: &[ActionSpaces]) {
    let a = moves_done(state, actions);
    let a = answer(state, &a, 1, vec![hidden("0xa")]);
    quiet(&a);
}

/// The capture and the hiding are one action, because two would be carried out
/// on a thread each and the capture would lose the race about half the time —
/// with nothing left on screen to photograph.
#[test]
fn minimizing_photographs_a_window_and_hides_it_in_one_go() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    assert!(matches!(only(&a), ActionSpaces::Look { .. }), "{a:#?}");

    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    let ActionSpaces::Minimize {
        geometry, thumb, ..
    } = only(&a)
    else {
        panic!("expected a minimize, got:\n{a:#?}");
    };
    assert_eq!(geometry.width, 800, "cropped to the window, not the screen");
    assert!(thumb.ends_with(".png"), "{thumb}");

    let tiles = s.minimized();
    assert_eq!(tiles.len(), 1, "one tile in the dock");
    assert_eq!(tiles[0].home, WorkspaceId(1), "it remembers where it was");
    assert_eq!(tiles[0].title, "Mozilla Firefox");
}

/// The rule the two halves of this module had to share a machine to get right.
/// A minimized window is not fullscreen and is not where it lives, and read by
/// the Space rules that is a window to put right — which would pull it straight
/// back out of the dock.
#[test]
fn the_space_rules_leave_a_minimized_window_where_it_is() {
    let mut s = minimized();

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![hidden("0xa")]);
    quiet(&a);
    assert_eq!(s.minimized().len(), 1, "still in the dock");
}

#[test]
fn clicking_the_tile_sends_it_home_and_drops_the_thumbnail() {
    let mut s = minimized();
    let thumb = s.minimized()[0].thumb.clone();

    let a = drive(&mut s, EventSpaces::RestoreRequested { addr: addr("0xa") });
    assert_eq!(a.len(), 2, "home, then forget the thumbnail:\n{a:#?}");
    assert!(
        matches!(
            a[0],
            ActionSpaces::Move {
                to: MoveTarget::Workspace(WorkspaceId(1)),
                ..
            }
        ),
        "{a:#?}"
    );
    assert!(
        matches!(&a[1], ActionSpaces::Forget { thumb: t } if t == &thumb),
        "{a:#?}"
    );
    assert!(s.minimized().is_empty(), "the tile is gone");
}

/// macOS greys the yellow button out on a fullscreen window, and this is the
/// same rule from the other side: minimizing out of a Space would leave the
/// Space behind, and the rule that brings a window home would spend the rest of
/// the session pulling against the one that keeps it hidden.
#[test]
fn a_window_in_a_space_cannot_be_minimized() {
    let mut s = in_a_space();

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    let a = answer(&mut s, &a, 9, vec![client("0xa", 9, true)]);
    quiet(&a);
    assert!(s.minimized().is_empty());
    assert!(s.in_space(&addr("0xa")), "and it is still in its Space");
}

/// The same sweep that prunes an origin, except a thumbnail is a file on disk,
/// so it has something to hand back.
#[test]
fn a_window_closed_while_minimized_takes_its_thumbnail_with_it() {
    let mut s = minimized();

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, Vec::new());
    assert!(matches!(only(&a), ActionSpaces::Forget { .. }), "{a:#?}");
    assert!(s.minimized().is_empty());
}

/// A daemon that died took the only record of where a hidden window came from.
/// Nothing on screen can ask for it back, so it is rescued to a default rather
/// than left stranded for the rest of the session.
#[test]
fn a_window_left_hidden_by_a_previous_run_is_rescued() {
    let mut s = StateSpaces::new(ConfigSpaces {
        fallback_home: WorkspaceId(3),
        ..Default::default()
    });

    let a = drive(&mut s, EventSpaces::Due);
    let a = answer(&mut s, &a, 1, vec![hidden("0xa")]);
    assert!(
        matches!(
            only(&a),
            ActionSpaces::Move {
                to: MoveTarget::Workspace(WorkspaceId(3)),
                ..
            }
        ),
        "{a:#?}"
    );
}

/// The tile is badged with the icon of the app it belongs to, which is the
/// dock's fact and arrives from above rather than being read.
#[test]
fn a_tile_is_badged_with_its_app_icon() {
    let mut s = StateSpaces::default();
    s.set_app_icons(vec![(
        "firefox".to_string(),
        "/icons/firefox.png".to_string(),
    )]);

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    assert_eq!(s.minimized()[0].icon, "/icons/firefox.png");
}

/// GTK caches a `background-image` by its path, so a reused name would draw the
/// window that was minimized before this one.
#[test]
fn no_two_thumbnails_share_a_path() {
    let mut s = minimized();
    let first = s.minimized()[0].thumb.clone();

    // Restore it, letting the move land: a look taken while one is in flight is
    // not believed, and the minimize below turns on one that is.
    let a = drive(&mut s, EventSpaces::RestoreRequested { addr: addr("0xa") });
    let a = moves_done(&mut s, &a);
    answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);

    // Then put the same window away again.
    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    assert_ne!(s.minimized()[0].thumb, first);
}

/// The bug the restore path had. A move that finishes leaves the module holding
/// a picture of the world from before it, and the rescue rule read that picture
/// — window still hidden, nobody remembering minimizing it — as a window
/// stranded by a previous run, and sent the one the user had just restored
/// straight back out again.
#[test]
fn a_restore_is_not_mistaken_for_a_stranded_window() {
    let mut s = minimized();

    let a = drive(&mut s, EventSpaces::RestoreRequested { addr: addr("0xa") });
    let a = moves_done(&mut s, &a);
    assert!(
        a.iter().all(|x| !matches!(x, ActionSpaces::Move { .. })),
        "nothing decided on the layout from before the move:\n{a:#?}"
    );

    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    quiet(&a);
}

/// The bug the first minimized tile had. The runtime publishes the variables a
/// turn leaves behind *before* it carries out that turn's actions, so a tile
/// naming its photograph straight away named a file grim had not written yet —
/// and GTK, having failed to load it, remembers the failure and never looks at
/// that path again.
#[test]
fn a_tile_names_its_photograph_only_once_there_is_one() {
    let mut s = StateSpaces::default();

    let a = drive(&mut s, EventSpaces::MinimizeRequested);
    let a = answer(&mut s, &a, 1, vec![client("0xa", 1, false)]);
    assert!(matches!(only(&a), ActionSpaces::Minimize { .. }), "{a:#?}");
    assert!(
        !s.minimized()[0].captured,
        "nothing has been photographed yet"
    );

    let a = drive(&mut s, EventSpaces::Captured { addr: addr("0xa") });
    quiet(&a);
    assert!(s.minimized()[0].captured);
}

/// A capture that failed still leaves a tile: it is how the window comes back,
/// and a window with no way back is worse than one with no picture.
#[test]
fn a_window_whose_photograph_failed_is_still_in_the_dock() {
    let mut s = minimized_without_a_photograph();
    assert_eq!(s.minimized().len(), 1);

    let a = drive(&mut s, EventSpaces::RestoreRequested { addr: addr("0xa") });
    assert!(
        a.iter().any(|x| matches!(x, ActionSpaces::Move { .. })),
        "{a:#?}"
    );
}
