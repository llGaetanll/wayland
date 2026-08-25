# Style

House rules a formatter cannot enforce. `rustfmt` owns everything mechanical;
this document owns the rest.

Where a rule has a reason, the reason is given — a rule nobody understands is a
rule that gets worked around.

## Comments

**Keep comments to a minimum.** Clarifications are welcome; a comment on every
line is not. The code already says what it does.

**No banner comments.** Not these:

```rust
// ── the dropdowns ───────────────────────────────────────────────────────────
```

They have to be re-padded when renamed, they encourage files to grow past the
point where they should have been split, and they carry no information a `mod`
or a heading in the file's doc comment does not. If a file needs sections, it
probably needs to be two files. No comment lines that are just dashes, either.

**Say why, not what.** A comment earns its place by explaining a decision, a
constraint, or a trap:

```rust
// Close before open: eww does not deduplicate, so asking it to open a window
// it already has leaves an orphaned surface behind.
```

**Doc comments on anything public**, and on private items whose purpose is not
obvious from the name. Start with a one-line summary that reads as a sentence.

## Panics

**Production code does not panic.** The daemon must not fall over because an
invariant we assumed turned out to be false in someone else's output, and the
machine must not take the bar down mid-session.

- No `assert!` / `assert_eq!` in non-test code.
- No `unwrap()` or `expect()` on anything that depends on input, IO, or data
  from a source. Return a `Result`, or handle the absent case.
- `expect()` is acceptable only for genuine programmer invariants that cannot
  depend on runtime data — a `Mutex` poisoned by another panic, a regex literal
  compiled at startup. Say why in the message.

**When a case is genuinely impossible, say so with `unreachable!`:**

```rust
let Some(menu) = self.open else {
    unreachable!("a menu was just opened")
};
```

Better than `expect()`, which reads as "this might fail and I am choosing to
die", and better than silently skipping, which hides a broken invariant behind a
missing widget. `unreachable!` says the case cannot happen and names the reason.
Reserve it for what is provable from the lines just above; anything that depends
on input or IO gets a `Result`.

**`debug_assert!` and `debug_assert_eq!` are encouraged**, especially in state
machines. They document an invariant, fail loudly in tests and debug builds, and
compile away in release. If you are tempted to write `assert_eq!` to check
state, write `debug_assert_eq!` instead.

## State machines

The shape and the rules are in [STATE_MACHINES.md](STATE_MACHINES.md). The two
that get broken most often: fields are private, and nothing derived is stored.

## Logging

**No `println!` or `eprintln!` anywhere.** Everything goes through `tracing`, so
every line gets a level, structured fields and the commit it came from. The
exception is a command whose output *is* the answer — `dump` writes to stdout.
How turns are recorded is in [STATE_MACHINES.md](STATE_MACHINES.md).

## Wire types

**Anything that turns state into bytes for something else is a `wire_*` file.**
`WireVars`, `WireJson`, and whatever serde types arrive later. Serialization is
not domain logic — nothing decides anything in it — but it is not IO either:
nothing there opens a socket or touches a file. It lives with the domain code,
prefixed so a directory listing never confuses the two.

### Testing them

Serialization and deserialization are the one place where a literal expected
value beats a pattern, because the literal *is* the contract. Two macros, and
nothing else:

**Serializing.** Two arguments: the value, and the one-line JSON it must
produce. No snapshot — the expected text is short enough to read in the diff,
and writing it out is the assertion.

```rust
serializes!(
    WifiRow { ssid: "cafe".into(), signal: 70, secure: true },
    r#"{"ssid":"cafe","signal":70,"secure":true}"#
);
```

**Deserializing.** Two arguments: the one-line JSON message, and a multi-line
inline snapshot of the value it must produce. This one expands to an inline
`insta` debug snapshot, so the expected value is generated rather than typed and
stays readable however deeply the type nests.

```rust
deserializes!(
    r#"{"ssid":"cafe","signal":70,"secure":true}"#,
    @r###"
    WifiRow {
        ssid: "cafe",
        signal: 70,
        secure: true,
    }
    "###
);
```

That is the only snapshot testing in the tree, and it is deliberate: the ban in
[STATE_MACHINES.md](STATE_MACHINES.md) is on asserting *behaviour* through a
`Debug` string nobody chose. Here the shape of the value **is** what the test is
about, and a hand-written expectation for a nested type is worse — long, wrong
often, and revised by hand every time a field moves.

Nothing in the tree deserializes JSON yet, so `insta` is not a dependency yet
either. The first type that needs it brings both.

## JSON

**Formatted with 4-space indentation.** A one-line file is unreadable in a diff
and unreviewable by a person, and these files exist to be read — they are the
test corpus.

```sh
python3 -m json.tool --indent 4 in.json out.json
```

**Committed only when small, and trimmed to what the test needs.** A captured
command output or API response is mostly fields we never parse; keeping them
bloats every diff that touches the file and lives in the history forever.

- **Drop fields nothing reads.**
- **Keep only as many records as the test needs.** If one entry proves the
  behaviour, commit one. Counts are the exception — a test asserting "four
  networks" needs all four.

A capture that is still large after both is a sign the test wants a hand-written
fixture instead.

## Before every commit

Both of these, every time:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
```

**No lints anywhere.** Not "no new lints" — none. A warning that is genuinely
wrong gets `#[expect(lint, reason = "…")]` with the reason written out, so the
next person knows it was considered rather than ignored.

## Imports

One `use` per item. Groups are std, then external, then workspace, then
`crate`.
