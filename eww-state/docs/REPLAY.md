# Replay and reduction

A vision, not a description. Nothing here is built yet; it is written down so
that the decisions made today do not quietly rule it out tomorrow.

Someday this program runs in prod and has a bug. That is fine. What matters is
what happens next: recovering the events and actions around that moment,
reproducing it, and shrinking it to something small enough to understand and
keep as a test. Every rule in [STATE_MACHINES.md](STATE_MACHINES.md) exists to
make that possible — this document says what it is possible *for*.

## The three steps

**Recover.** `eww-state trace --since … --until …` reads the rotating log and
prints the turns in that window: the events that went in, the actions that came
out. This is nearly free — the runtime already logs both halves of every turn,
paired by `turn=` and stamped with the commit.

**Reproduce.** Feed that trace back through the machine at the same commit and
check that the actions it produces are the ones the log recorded. If they match,
the recording is faithful and the bug is now a thing that happens on demand. If
they do not, there is a second bug — a decision depending on something that is
not an event — and that one is worse.

**Reduce.** Given a trace and a statement about it that must stay true (an
action we did not expect, an invariant that broke), find the shortest prefix of
events that still makes it true. Not by hand: by machine, the way a property
test shrinks a counterexample.

The end of that chain is the point. The reduced trace is printed as an `eval!`
block, pasted into `tests/`, and the bug can never come back.

## What replay demands

Determinism is not a nice property here, it is the whole mechanism, and it is
easy to lose one input at a time.

**Every input is an event.** Time, randomness, IO results, the compositor's
opinion — all of it arrives through `handle_events` or it is invisible to a
replay. This is rule 2 and it is the one that erodes: an argument added to a
function "just for now" is an input that never appears in the log.

**The trace has to be machine-readable.** Today events and actions derive
`Debug` and nothing else, which is right for a human reading a log and useless
for feeding one back in. Something has to close that gap, and the options are:

- Parse the `Debug` text. Cheap to start, fragile forever — the format is
  nobody's contract and changes when a field does.
- Derive `serde` on events and actions and write a second, structured stream
  next to the human log. Honest cost: `Serialize` spreads to every payload type,
  which is exactly the objection rule 5 raises against `PartialEq`. The
  difference is that this derive buys a capability rather than a convenience,
  and it does not leak into how tests are written — `eval!` still matches
  patterns and still needs no trait at all.

The second is the one to take, when it is taken. Worth doing at the same time:
**`Instant` cannot survive a process boundary.** It is opaque, monotonic from an
arbitrary origin, and meaningless in another run. The tick should carry a
plain `Millis(u64)` measured from a recorded epoch, which serializes, replays,
and reads better in a log besides.

**A trace needs a starting state.** The log is a ring buffer, so a window from
the middle of a session begins nowhere in particular. Two ways out: keep the
whole session's events (unbounded, so no), or write a state snapshot at every
rotation and at intervals, and start a replay from the most recent one before
the window. The config counts as part of that snapshot — `StateBar::new` takes
one, so two replays with different configs are two different programs.

**Nothing derived may depend on iteration order.** Rule 3 says derived data is
computed on the way out; if that computation walks a `HashMap`, two replays of
the same events can emit the same set of actions in a different order, and a
comparison against the log will fail for no reason.

## The reducer

A separate binary sitting above this one. It takes a trace, a commit, and a
**property** — something about the run that must stay true — and returns the
smallest event sequence that still satisfies it.

**The algorithm is delta debugging, not random shrinking.** Property tests
shrink because they generated the input; here the input is a real recording, so
the right tool is `ddmin`: split the event list into chunks, test each
complement, keep any that still satisfies the property, and increase granularity
when a pass removes nothing. It converges to a 1-minimal trace in `O(n log n)`
replays, and a replay is microseconds of pure computation.

**Removal is always legal, which is why this works at all.** A machine here
accepts any event in any state — a probe reply with no request, a close with
nothing open, a tick out of nowhere. Reduction leans entirely on that: chunks
can be deleted without producing a trace the machine would refuse. A machine
written to assume a handshake would be unshrinkable, and it would be a worse
machine for other reasons too.

**Then shrink inside the events.** Once no whole event can go, the payloads can:
a percentage toward zero, a list toward empty, a string toward the shortest one
that still holds the property. This is the proptest-shaped half, and it is what
turns *a wifi scan with 34 networks* into *a wifi scan with one*.

### Properties

The simple case — "this action must still be emitted" — is a pattern, and the
CLI should take it as one, since that is the vocabulary `eval!` already uses:

```
eww-state-reduce --trace turn-17-window.jsonl --at 6e917a6 \
  --preserve 'Action::ToUi(ActionUi::OpenWindow("menu-bt"))'
```

Real bugs are rarely that tidy, so the property has to be able to say more:
these two actions in one turn, this action never followed by that one, this
state invariant false at the end. The general form is a predicate over the whole
replay — every turn's events, actions, and the state after — and the escape
hatch is to let the property be a Rust function compiled against the crate,
with the pattern form as sugar over the common case.

**One trap, and it bites immediately.** A bug is often a *missing* action, and
"action A never appeared" is satisfied by the empty trace — reduction will
happily delete everything and report that as minimal. An absence property must
be anchored to the thing that should have caused it: *event E occurs, and no
action A follows it*. Absence alone is not a property, it is a way to make the
reducer agree with you about nothing.

### Generic over the machine

None of this is about widgets. The reducer needs a machine that steps events to
actions, a trace, and a property; that is the whole interface, and it is worth
building against a trait rather than against `StateBar`:

```rust
trait Machine {
    type Event;
    type Action;
    fn step(&mut self, events: &[Self::Event], actions: &mut Vec<Self::Action>);
}
```

Any program obeying the rules in [STATE_MACHINES.md](STATE_MACHINES.md) can be
reduced by the same tool. That argues for the reducer living in its own crate
with no dependency on this one.

**The commit is not decoration.** A reduction has to run the code the bug
happened on, which is the commit stamped on every log line. The clean shape is
that the reducer never links the machine at all: it checks out that commit,
builds it, and talks to a `replay` subcommand over stdin and stdout — trace in,
actions out. Language-agnostic, version-agnostic, and a persistent child process
keeps the per-candidate cost near zero.

## What to build, in order

1. **Hold the line on the rules.** Free, and everything else depends on it. Every
   input an event, every turn logged, no clock in the machine.
2. **`Millis` instead of `Instant`, and a structured trace stream.** The two
   changes that unlock the rest.
3. **`eww-state trace` and `eww-state replay`.** Extract a window; re-run it;
   diff the actions against the log. Ship this before any reducer — a replay
   that diverges is a determinism bug, and finding those is worth more than
   minimising anything.
4. **`ddmin` over a pattern property, printing an `eval!` block.** The first
   version that pays for itself.
5. **Payload shrinking, state snapshots, richer properties.** In that order,
   as real bugs ask for them.

## Honest reservations

**Minimal is not the same as explanatory.** `ddmin` optimises for length, and
the shortest trace that reproduces a bug can be a strange one — three ticks and
a probe reply, with the click that made it make sense deleted because the bug
survives without it. It is still the right thing to hand a person or a model;
just do not expect it to read as a story.

**The property is the hard part.** "That action is not what I expected" is a
sentence, not a predicate, and turning one into the other is most of the work of
using this tool. Expect the first few reductions to be as much about discovering
what you actually meant as about shrinking anything.

**Faithful recording is worth more than reduction.** If only one of these gets
built, build step 3. A bug you can reproduce at will is already most of the way
fixed; a bug you can reproduce *and* shrink is a luxury on top of that.
