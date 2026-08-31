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
use module_spaces::EventSpaces;
use module_spaces::MoveTarget;
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
        fullscreen,
        monitor: 0,
    }
}

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
            ActionSpaces::Move { id, .. } => Some(*id),
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
