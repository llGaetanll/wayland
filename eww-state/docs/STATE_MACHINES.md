# State machines

What we learned building state machines in another project, written down here
because `crates/core` is the same design and the same traps apply.

## Keep state and IO apart

This is the rule everything else follows from. The machine decides, the runtime
acts, and the machine never learns how the runtime does it.

- **IO** — probes, sockets, `eww`, the compositor. Plain functions over plain
  data: run the command, parse the output, map it onto the machine's own types.
  Nothing here decides anything.
- **State** — the machine. Synchronous, no clock, no sockets, no files. It owns
  every rule.

A parser can be run from a test, a rule can be exercised without a compositor,
and neither is entangled with the other.

## Events in, actions out

1. Something happens — a click, a probe returns, a timer fires. That is an
   **event**: a plain enum value, nothing more.
2. The event goes into the machine, which holds all the state and updates it
   synchronously. It reads no clock, opens no socket, awaits nothing.
3. What it hands back are **actions**: the IO. A thin runtime carries them out,
   and whatever comes back enters as the next event.

So the loop is `event -> machine -> action -> IO -> event`, and the only part
with a decision in it is the one part that cannot touch the outside world.

### Why

The payoff is that a log of the events is a recording of the session. Replay it
and you get the same state and the same actions — so a bug reported with no
repro can still be found, fixed, and kept as a test. A bug seen once never comes
back. If a decision can instead depend on a clock, a race, or which of two
things happened to finish first, none of that holds and you are guessing. That
recording is not hypothetical — see [Logging the turn](#logging-the-turn), and
[REPLAY.md](REPLAY.md) for where that recording is meant to lead.

## The rules a machine follows

These are settled. A new machine follows all of them.

1. **One entry point, and it is the only way state changes.** The fields are
   private. A `pub` field is a way to mutate state without an event, which is
   the one thing this design exists to prevent.

2. **Time arrives as an event.** The clock is read at the edge, by the runtime,
   and sent in as an ordinary event like any other. Nothing inside reads one —
   that is what makes a replay land where it did the first time. Same for
   randomness and IO. See [Waiting without sleeping](#waiting-without-sleeping)
   for why it is an event rather than an argument.

3. **Nothing derived is stored.** Filtering, grouping, totals, the variable
   table, the window list, the poll plan — computed on the way out. A field that
   could be recomputed is a field that can disagree with itself. Most of the old
   bugs lived exactly there.

4. **Make illegal states unrepresentable.** `Option<MenuId>`, not a boolean per
   menu: two-open then does not exist as a value, so no sequence of events can
   reach it. Prefer this over a rule that has to be enforced everywhere.

5. **Derive as little as possible on events and actions.** `Debug` for logging
   and failure messages; `Clone` only where something must own one. Not
   `PartialEq` — tests should match on patterns, and that derive spreads a bound
   through every type it touches.

6. **No panics.** `debug_assert!` for invariants, `unreachable!` where a case is
   provably impossible, `Result` for anything depending on input or IO. See
   [STYLE.md](STYLE.md).

7. **Split by ownership, not by screen.** A sub-machine is worth it when one
   widget owns state nothing else reads. Shared data stays on the parent — two
   copies are two copies that can disagree. Sub-machines do not handle events
   themselves: the parent routes to them, so there is still exactly one way
   anything changes. A widget that needs something of its parent asks; the
   parent decides.

8. **Anything that is a *rule* belongs in the machine, even when it looks like
   IO's business.** "May I send this yet?", "should this window be open?", "how
   often should this be sampled?" are all arithmetic over state, so they live in
   the machine rather than being guessed at by the runtime.

## Waiting without sleeping

A machine often needs to wait: back off before retrying, hold a notification for
five seconds, do not re-sample more than once a minute. That is a rule, so by
rule 8 it belongs in the machine — but sleeping is IO, and the machine cannot do
IO.

**The runtime ticks; the machine decides.** Once a second the runtime reads the
system clock and sends it in as an ordinary event:

```rust
enum EventClock {
    Tick { now: Instant },
}
```

The machine stores it and answers *can I do this yet* from what it holds. The
tick is what keeps that stored time moving when nothing else is happening.

**The time is a payload, not an argument to `handle_events`.** An argument is
invisible from the outside: it never appears in the event log, so a recorded
session cannot be replayed and a bug that depended on the clock cannot be
reproduced. As an event it is recorded like everything else, and the replay
lands where it did the first time.

Every waiting rule is then arithmetic the machine can do on the spot:

```rust
if self.now.duration_since(self.last_sampled) >= self.interval {
    actions.push(Action::ToProbe(ActionProbe::Sample { what }));
    self.last_sampled = self.now;
}
```

So "wait five seconds, then do X" is never a sleep anywhere. It is *X has not
happened yet and now is not yet five seconds past* — asked again on every tick,
answered the same way every time it is replayed.

**A second is close enough.** The machine's clock is a rounded copy of the real
one, and that is deliberate: a rule needing finer resolution than the tick is a
rule about to depend on scheduling luck. Pick the tick to suit the coarsest
thing that has to feel responsive and leave it there.

**A test moves time the same way a program does.** There is no back door for
tests, no `set_now`: a test that is *about* time sends the tick it wants, which
is the same event the runtime would have sent.

## The shape of the types

Events and actions are the vocabulary the whole program shares, so name where
they came from and where they are going:

```rust
enum Event {
    FromUi(EventUi),          // a click, a menu toggled
    FromProbe(EventProbe),    // a snapshot arrived, a command failed
}

enum Action {
    ToUi(ActionUi),           // set these vars, open that window
    ToProbe(ActionProbe),     // sample this now
}
```

Grouping by origin keeps a match readable as the program grows, and makes a
recorded log read as a story rather than a flat list.

## The shape of the machine

The machine owns its state and mutates it in place. Actions are not returned;
they are pushed into a buffer the caller owns, so a batch of events costs one
allocation and a replay can reuse the same one:

```rust
impl State {
    pub fn handle_events(&mut self, events: &[Event], actions: &mut Vec<Action>) {
        for event in events {
            self.handle_event(event, actions);
        }
    }

    fn handle_event(&mut self, event: &Event, actions: &mut Vec<Action>) {
        match event {
            Event::FromUi(e) => self.handle_event_ui(e, actions),
            Event::FromClock(e) => self.handle_event_clock(e, actions),
            Event::FromProbe(e) => self.handle_event_probe(e, actions),
        }
    }

    fn handle_event_ui(&mut self, …) { … }
    fn handle_event_probe(&mut self, …) { … }
}
```

`handle_events` is the only public entry point; everything under it is one
function per event group, mirroring the enums. `handle_event` does nothing but
dispatch, so the arms stay small and each group's rules sit in one place.

## The runtime on the other side

Every program is two pieces: a machine that decides and a runtime that acts. The
runtime mirrors the machine's shape, so an action and the thing carrying it out
sit at the same depth:

```rust
impl Runtime {
    pub fn run() { … }                                 // the only public fn

    fn perform_actions(&self, actions: &[Action]) { … }
    fn perform_action(&self, action: &Action) { … }    // dispatch only
    fn perform_action_probe(&self, action: &ActionProbe) { … }
}
```

One turn is: collect events, hand them to the machine, carry out the actions,
log both. Whatever the IO produces re-enters as the next event, which closes the
loop. Only `run` is public — everything else is reachable solely by carrying out
an action, so nothing can do IO without an action asking for it.

The runtime is also where reconciliation belongs: diff the derived vars against
what was last published, diff the derived windows against what is open, retune
the intervals. The machine says what should be true; the runtime makes it true
without remembering anything twice.

## Logging the turn

A replay is only as good as the recording, so the runtime writes down every turn
it takes: the events that went in, and the actions that came back out.

**Two lines per turn, paired by an id.** The events vector is one line, the
actions vector it produced is the next, and both carry the same `turn=` so a log
read months later still says which batch answered which. One line each, not one
per element — the batch is the unit that matters, because it is the unit
`handle_events` was called with and the unit a replay would feed back in.

```
2026-08-24T18:42:03.401Z 6e917a6 INFO  state -> events turn=17 n=2 [MenuToggle(Wifi), MenuToggle(Bt)]
2026-08-24T18:42:03.402Z 6e917a6 INFO  state <- actions turn=17 n=3 [CloseWindow("menu-wifi"), OpenWindow("menu-bt"), SetVars(…)]
```

`->` and `<-` mean only *what went in* and *what came out*. Anything the runtime
then makes of an action is a decision of its own and gets its own line.

**Every line carries the build.** The first six characters of the git sha, from
a `build.rs` that shells out to `git rev-parse --short HEAD` and exports it as
an env var the binary reads with `env!`. Two `rerun-if-changed` lines on
`.git/HEAD` and `.git/refs` keep it honest, and a tree with no git at all still
builds — it logs `unknown` rather than nothing. A log without the commit is a
log that cannot be lined up against a source tree, and the build a report
arrives for is always one nobody has checked out.

**Everything goes through `tracing`. Nothing prints.** No `println!`, no
`eprintln!`, no hand-rolled logger, anywhere — a print is a line that skips the
level, the fields, the file and the sha, and it is exactly the line missing from
the log you are handed. The one exception is a command whose *output is the
point*: `eww-state dump` writes its answer to stdout, because that is the
program's result rather than a record of it. Everything else is an event with
structured fields, and spans give the surrounding context for free.

**The file rotates in place.** Logs go somewhere appropriate for the platform —
`$XDG_STATE_HOME/eww-state/` — capped at a maximum size and rotated, so the
oldest turns fall off the end as new ones arrive. A ring buffer on disk: bounded
forever, and always holding the most recent history, which is the part a bug
report needs.

**`Debug` is what makes this possible**, which is why it is the one derive
events and actions always carry (rule 5).

## Naming and layout

The prefix goes first in both the type and the file name, so a directory listing
sorts by kind and says what it is at a glance:

| Thing | Type | File |
| --- | --- | --- |
| The machine | `StateBar`, `StateWifi` | `state_bar.rs`, `state_wifi.rs` |
| The IO half | `RuntimeDaemon` | `runtime_daemon.rs` |
| Events | `EventUi`, `EventProbe` | `event_ui.rs`, `event_probe.rs` |
| Actions | `ActionUi`, `ActionProbe` | `action_ui.rs`, `action_probe.rs` |
| Wire types | `WireVars` | `wire_vars.rs` |

**One event or action group per file.** A file per group scales; a single
`events.rs` does not.

Spell words out: `server-main`, not `srv-main`.

## Testing

A test is a slice of events in, a `Vec<Action>` and a state to assert on out —
no runtime, no await, no mocks. Machines are testable precisely because they are
boring.

Written by hand, though, that shape disappears: every step is a `Vec::new()`, a
`handle_events` call, a length check and some index arithmetic, and the
conversation the test is *about* is buried in it. So the other project drives
machines through one macro instead.

### `eval!`

```rust
eval!(&mut state,
    < EventUi::Started,
    > Action::ToProbe { action: ActionProbe::Sample { what: Probe::Wifi } },
    > Action::ToUi { action: ActionUi::Render { .. } },

    < EventUi::MenuToggled { menu: MenuId::Wifi },
    < EventUi::MenuToggled { menu: MenuId::Bt },
    > Action::ToUi { action: ActionUi::Render { .. } },
);
```

`<` is an event going in, `>` is an action expected out. The whole vocabulary is
those two symbols; there is nothing else to learn.

**Adjacent events are one batch.** A run of `<` lines goes into a single
`handle_events` call, and the `>` lines after them are exactly what that call
must produce — in order, nothing extra, nothing missing. A `<` after a `>`
starts the next batch. That is what lets a test say *these two clicks land
together* as opposed to *one, then the other*, which is usually the difference
between the bug and the fix.

**A trailing batch with no `>` asserts silence.** Zero expected actions is a
real assertion, not an omission: the events were accepted and the machine
deliberately did nothing. It only reads that way at the end, though — since a
`<` joins the open batch, mid-block silence has to be its own `eval!`.

**The `>` lines are patterns, not values.** `..` and `_` cover whatever the test
does not care about:

```rust
> Action::ToProbe { action: ActionProbe::Sample { .. } }
```

This is the load-bearing part. The macro expands to `matches!`, and pattern
matching needs no trait at all — which is how events and actions stay free of
`PartialEq` (rule 5). An equality-based harness would drag that derive through
every type they touch, and would also force every test to spell out fields it
has no opinion about.

**Time needs no syntax of its own.** It is an event, so a test that is *about*
time — staleness, backoff, a debounce — sends one:

```rust
eval!(&mut state,
    < EventClock::Tick { now: T0 + Duration::from_secs(FRESH_FOR) },
    < requested(false),
    > Action::ToProbe { action: ActionProbe::Sample { .. } },
);
```

That is the same event the runtime sends, so the test drives the machine exactly
as the program does, with no test-only door into its state.

**Everything else is a plain assertion after the macro.** `eval!` covers the
actions; the state the machine is left in is checked the ordinary way:

```rust
eval!(&mut state, … );

assert_eq!(state.entries().len(), 2, "the ledger must not empty itself");
```

That split is deliberate. The macro owns the one thing that is awkward by hand —
batching and ordered pattern checks — and stays out of the way of everything a
normal `assert!` already does well.

### How it is built

A proc macro, a couple of hundred lines. Worth knowing because the decisions are
small and each one shows up in the failure messages:

- **Parsing.** `<` parses the rest of the line as a `syn::Expr`, `>` as a
  `syn::Pat` (`Pat::parse_multi_with_leading_vert`). Parsing the two sides
  differently is the whole trick: an event is a value you construct, an action
  is a pattern you match.
- **Batching happens at parse time.** The parser keeps a `Vec<Batch>`; a `<` on
  a batch that already has actions closes it and opens a new one. No runtime
  bookkeeping.
- **Count first, then patterns.** Each batch expands to a `assert_eq!` on
  `actions.len()` before any `matches!`, so a missing or extra action reports as
  *that*, rather than as a mismatch three lines further down.
- **One check per action, written out.** The checks are unrolled rather than
  looped so a failure can name the batch, the position, the pattern source and
  the `{:#?}` of what actually arrived.
- **Tidy the pattern text.** `quote!(#pat).to_string()` puts a space around
  every token, so a message would read `Action :: ToUi { action : … }`. A couple
  of `replace` calls on `" :: "`, `" : "` and `" , "` fix it. This is cosmetic
  and it is the only thing anyone reads when a test breaks.

An `eval!` here would take `Intent`/`Obs` in and `Action` out, and the same
shape applies unchanged.

### The rest of it

**No snapshot testing.** Not inline, not in files. A snapshot asserts on a
string, so it passes or fails on formatting and `Debug` output nobody chose
deliberately. A pattern says what the test actually cares about and ignores the
rest.

**Tests may `assert!` freely.** A one-line assertion on a value is usually the
clearest thing to write. What assertions are not for is driving a machine.

**Test the bugs you had.** The scenarios worth writing down are the ones that
actually happened: menus mashed, a drag racing a poll, a restart mid-flight.
Each one becomes a rule that cannot regress.
