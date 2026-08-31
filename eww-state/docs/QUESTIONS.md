# Open questions

Patterns in this codebase that are worth a closer look — either because they
were adopted quickly, or because they might be more machinery than the problem
needs. Not bugs. Each entry says where to look and what to be suspicious of.

Delete an entry once it has been looked at and the answer is either written down
somewhere better or acted on.

## `vars()`

**Where:** `Module::vars` in `crates/api-core/src/module.rs`, every module's
implementation, `StateBar::vars`, and `Reconciler::sync_vars` in
`crates/daemon/src/reconcile.rs`.

Each module writes its eww variables into a shared `WireVars` table; the bar
concatenates all nine; the runtime diffs the result against what it last
published and sends only what changed.

Worth questioning:

- **The names are strings, and the contract with `eww.yuck` is unchecked.**
  Rename a `defvar` and nothing fails to compile; the widget just stops
  updating. Two modules could quietly claim the same name, and the last one to
  write wins.
- **It is recomputed in full after every batch**, including the whole network
  list as JSON, to usually discover nothing changed. Cheap at this size, but it
  is a full serialize per turn.
- **`&self` and pure is load-bearing** — it is what stops the displayed value
  drifting from the real one — but nothing enforces it beyond the signature.
- **The one rule that is not obvious from the type:** never publish a path to a
  file that does not exist yet. GTK caches a load that failed, and the diff only
  sends a var when it changes, so one bad path is permanent. That is why
  `batt_icon` waits for the runtime to confirm the render.
