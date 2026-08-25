# CLAUDE.md

- Be concise in your responses.
- If a question can be answered by looking at the code, look at the code instead
  of asking the user.

## Docs

- `docs/STATE_MACHINES.md` — how `crates/core` is built: events in, actions out,
  the rules a machine follows, how turns are logged, and how machines are
  tested. Read it before changing state.
- `docs/STYLE.md` — the house rules a formatter cannot enforce: comments, no
  panics in production code, JSON fixtures, imports. Read it before writing
  code.

The two that catch people most often:

- **No `assert!`/`unwrap()`/`expect()` in production paths.** `debug_assert!` is
  encouraged in state machines instead.
- **No banner comments** (`// ─── section ───`). Split the file instead.
