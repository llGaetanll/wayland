//! Telling a variable table we filled from one that was emptied under us.
//!
//! `eww reload` re-reads the config and puts every `defvar` back at its
//! default, on the same process and the same socket — so the inode the server
//! is named by is unchanged and nothing dies. The machine stamps a generation
//! into a var of eww's own and the probe reads it back; these are the
//! conversations that stamp can have.

use api::millis::Millis;
use eval::eval;
use module_bar::event_clock::EventClock;
use module_bar::Action;
use module_bar::ActionBar;
use module_bar::ConfigBar;
use module_bar::Event;
use module_bar::EventBar;
use module_bar::EwwGen;
use module_bar::ServerId;
use module_bar::StateBar;
use module_bar::EWW_GEN_VAR;

/// Longer than [`EWW_STAMP_SETTLE`], which is private: a read arriving after
/// this much has passed is one the machine has to believe.
const SETTLED: u64 = 11_000;

fn bar() -> StateBar {
    StateBar::new(ConfigBar::new("/icons", "/cache", "/thumbs"))
}

/// What the probe found: who is listening, and what their table is stamped
/// with. The same server throughout — a takeover is a different test.
fn seen(gen: Option<EwwGen>) -> Event {
    Event::Bar(EventBar::EwwObserved {
        server: Some(ServerId::new(1)),
        gen,
    })
}

fn tick(ms: u64) -> Event {
    Event::Clock(EventClock::Tick { now: Millis(ms) })
}

/// The stamp as eww would read it back, taken from the var table the runtime
/// publishes rather than from anything private. This is the round trip.
fn stamp(bar: &StateBar) -> Option<EwwGen> {
    bar.vars().get(EWW_GEN_VAR).and_then(EwwGen::parse)
}

/// The ordinary state of things: our stamp comes back, over and over, and
/// there is nothing to do about it.
#[test]
fn a_table_carrying_our_stamp_is_left_alone() {
    let mut b = bar();
    eval!(&mut b,
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    let ours = stamp(&b);
    assert!(ours.is_some(), "adopting stamps the table");

    eval!(&mut b,
        < seen(ours),
    );
    eval!(&mut b,
        < tick(SETTLED),
        < seen(ours),
    );
}

/// The bug. eww reloaded under us: same server, and every var back at the
/// default it was declared with. Nothing about the socket can see this, and
/// the stamp coming back empty is the whole of the evidence.
#[test]
fn a_reload_is_adopted_though_the_server_never_changed() {
    let mut b = bar();
    eval!(&mut b,
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    let before = stamp(&b);

    eval!(&mut b,
        < tick(SETTLED),
        // `None` is the `defvar` default, which we never publish.
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    assert_ne!(
        stamp(&b),
        before,
        "adopting again must stamp again, or the next read cannot be judged"
    );
}

/// The probe samples on its own thread, so a read can be taken before a stamp
/// is published and arrive after it. It reports a generation we have already
/// moved past, and the one thing that must not happen is stamping again —
/// the next stale read would say the same and it would never end.
#[test]
fn a_read_from_before_the_stamp_landed_is_not_a_second_reset() {
    let mut b = bar();
    eval!(&mut b,
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    let first = stamp(&b);

    eval!(&mut b,
        < tick(SETTLED),
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );

    // Both flavours of stale: the empty table from before the republish, and
    // the generation it replaced.
    eval!(&mut b,
        < tick(SETTLED + 1),
        < seen(None),
    );
    eval!(&mut b,
        < tick(SETTLED + 2),
        < seen(first),
    );
    // And then the read that was taken after it landed.
    let now = stamp(&b);
    eval!(&mut b,
        < tick(SETTLED + 3),
        < seen(now),
    );
}

/// Holding off is not giving up. A republish can fail quietly — the command
/// times out, the server is wedged — and an unstamped table that is still
/// unstamped a settle later is asked again, at that pace and no faster.
#[test]
fn a_table_that_stays_empty_is_stamped_again() {
    let mut b = bar();
    eval!(&mut b,
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );

    eval!(&mut b,
        < tick(SETTLED),
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    // Same probe interval as the real one: too soon to mean anything.
    eval!(&mut b,
        < tick(SETTLED + 5_000),
        < seen(None),
    );
    eval!(&mut b,
        < tick(2 * SETTLED),
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
}

/// A server that replaced another is a stranger twice over: an inode we have
/// not published to, holding a table we have not stamped. One adoption, not
/// two — the two signals answer the same question.
#[test]
fn a_takeover_and_an_empty_table_are_one_adoption() {
    let mut b = bar();
    eval!(&mut b,
        < seen(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );

    let stranger = |gen| {
        Event::Bar(EventBar::EwwObserved {
            server: Some(ServerId::new(2)),
            gen,
        })
    };
    eval!(&mut b,
        < tick(SETTLED),
        < stranger(None),
        > Action::Bar(ActionBar::EwwAdopt { .. }),
    );
    // Silence now says which server was adopted: it is the new one's table
    // that is carrying the new stamp.
    let now = stamp(&b);
    eval!(&mut b,
        < tick(SETTLED + 1),
        < stranger(now),
    );
}
