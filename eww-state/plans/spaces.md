# Spaces

Move the macOS-style fullscreen "Spaces" out of the compositor config and into
`crates/core` as a sub-machine, the way the widgets already were.

## What exists today

Outside this repo, in `~/.config/hypr`:

| where | what it does |
| --- | --- |
| `hyprland.lua`, the `__mac_spaces_init` block | the enter/exit rules, an echo debounce, animation toggling |
| `scripts/prune-spaces.sh` | deletes origin files whose window has closed |
| `scripts/mac-fullscreen.sh` | the close path: hop home, then close |

The rule is one sentence — *a window entering fullscreen moves to its own empty
workspace and returns to where it came from when it leaves* — and it is spread
across a Lua closure, two shell scripts and a directory of files.

The state it keeps:

1. **`$XDG_RUNTIME_DIR/mac-fs/<addr>`**, one file per window holding the
   workspace it came from. On disk because Lua has nowhere else to put it, read
   by both scripts, deleted from three code paths, and swept by a pruning loop
   over `hyprctl clients`. Stale entries accumulate.
2. **`gen[addr]`**, a per-window generation counter plus a 140 ms timer, there
   only to swallow the spurious `fs=0` then `fs=1` Hyprland emits when a
   fullscreen window is moved. Acting on those echoes bounces the window
   forever.
3. **`bars.lock`**, a flock serialising back-to-back events.
4. **Global animation state** — `move_instant` disables the workspaces
   animation, dispatches, and re-enables it on a deferred 100 ms timer.

Three timings are load-bearing: 140 ms debounce, 250 ms grow-wait, 100 ms
animation restore. That is the class of bug this project was started to remove.

Bar and dock visibility used to live here too and no longer does — the daemon
watches the same compositor socket and derives it from `StateBar`. That
cutover already happened; this plan is the rest of it.

## The idea the design turns on

**The echoes are ours.** Hyprland re-emits `fullscreen` because *we* moved the
window. A machine that knows it has a move in flight for an address does not
need a timer to tell the echo from a real toggle — it has the fact directly. The
debounce is not ported; it is deleted, and with it the only sub-second rule in
the machine.

The other two timings are not rules either:

- **The 100 ms animation restore** is part of how a move is performed, not a
  decision about when to perform it. It goes into the runtime's handler for one
  `MoveWindow` action: disable, dispatch, restore. The machine never learns the
  animation exists.
- **The 250 ms grow-wait** is cosmetic — it let the grow animation play before
  the cut, back when the bar's reserved area had to settle first. That
  precondition is gone. Build without it (stage 2), look at it (stage 5), and
  only if it visibly regresses put a delay in the runtime's action handler with
  a comment saying it is cosmetic. It does not become machine state.

So: three timers in, zero out. Nothing in `StateSpaces` reads `Ctx::now()`, and
nothing needs a tick finer than the 1 s one in `ticker.rs`.

## Shape

A sub-machine, `state_spaces.rs`, owning what nothing else reads:

```rust
pub struct StateSpaces {
    /// Where each window in a Space came from. Absent means "not in a Space",
    /// which is the whole of the old origin-file directory.
    origins: BTreeMap<WindowAddr, WorkspaceId>,
    /// The last snapshot, so a decision is made against settled state rather
    /// than against the event that woke us.
    clients: Vec<Client>,
    focused: WorkspaceId,
    /// Moves we asked for and have not seen finish. A fullscreen change for an
    /// address in here is our own echo.
    moving: BTreeMap<WindowAddr, JobId>,
}
```

`origins` replaces the file directory, `moving` replaces `gen` and the
debounce, and the flock has no counterpart: one thread, one machine, events
applied in order.

Rules, evaluated in `emit_actions` against the settled snapshot:

- **Enter** — a client is fullscreen, has no origin, and no move in flight:
  record its current workspace as its origin and emit `MoveWindow { to: Empty }`.
- **Exit** — a client is not fullscreen, has an origin, and no move in flight:
  drop the origin and emit `MoveWindow { to: Id(origin) }`. Emptying the Space
  makes Hyprland destroy it.
- **Gone** — an address with an origin is absent from the snapshot: drop the
  origin. This is `prune-spaces.sh`, as three lines that cannot leave a stale
  entry because there is no file to leave behind.
- **Close** — `EventUi::WindowCloseRequested`: if the focused window has an
  origin, `FocusWorkspace { origin }` first, then `CloseWindow`. A Space that is
  emptied while still focused lingers instead of being destroyed.

Both no-op cases from the Lua comment stay no-ops and now need no comment:
fullscreen with an origin is already in a Space, and not-fullscreen without one
is an ordinary window toggling maximize.

`debug_assert!` that no address is in both `moving` and a fresh enter.

## Alphabet

Additions only; nothing existing changes shape.

**`model.rs`** — `WindowAddr(String)` and `WorkspaceId(i32)` newtypes, and
`Client { addr, workspace, fullscreen }`. Our own types, not the compositor's
JSON: rule 9, and `REPLAY.md` depends on it.

**`Probe::Spaces`** replacing `Probe::Fullscreen`, carrying the whole picture:

```rust
EventProbe::Spaces { focused: WorkspaceId, clients: Vec<Client> }
```

`StateBar.fullscreen` is then derived from the same snapshot — *the focused
workspace holds a fullscreen client* — so bar visibility and Space placement can
never disagree about what the compositor said. One event, two readers, no
cross-talk between the machines.

**`EventUi::WindowCloseRequested`**, which is `mac-fullscreen.sh` entire.

**`ActionShell`** gains `MoveWindow { addr, to: MoveTarget }` where `MoveTarget`
is `Empty | Id(WorkspaceId)`, `FocusWorkspace { id }`, and `CloseWindow { addr }`.

**`StateId::Spaces`**, so move jobs route their completion home.

## Stages

Each stage is a commit that compiles, passes `cargo test`, and is worth having
on its own.

### 0. Pre-flight

`git status` and `cargo test`. A refactor was in flight when this was written —
`Instant` becoming `Millis`, and `State::handle_event` losing its `Ctx` so
actions come only from `emit_actions`. **Do not finish, fix or work around
somebody else's half-landed refactor.** Build against whatever the `State` trait
and `Ctx` look like when you pick this up; if the tree does not compile, stop
and say so.

### 1. The snapshot probe

`Probe::Spaces` and its event, `model.rs` types, `probe_spaces.rs` in the daemon
parsing `hyprctl clients -j` and `hyprctl activeworkspace -j`, and
`StateBar.fullscreen` fed from the new event. `hypr.rs` keeps the socket watch
and asks for `Probe::Spaces` instead of `Probe::Fullscreen`.

Fixtures per `STYLE.md`: real `hyprctl` output, 4-space indented, fields nothing
reads dropped, as few clients as the test needs.

*Done when:* behaviour is unchanged and every existing test still passes. The
bar hides on fullscreen exactly as before, now from a richer probe.

### 2. The machine, alone

`state_spaces.rs` with no wiring: `State` impl, the four rules, unit tests
through `eval!`. No `vars()` — this machine publishes nothing to eww.

*Done when:* these read as tests, and pass.

- entering fullscreen records the origin and moves to an empty workspace
- the echo after our own move produces no action at all (the case the 140 ms
  debounce existed for — assert silence, per the trailing-batch rule)
- leaving fullscreen moves back to the recorded origin
- a window that vanishes while in a Space leaves no origin behind
- close-in-a-Space focuses the origin before closing, close outside one does not
- two windows in Spaces at once do not interfere

### 3. Actions and runtime

`ActionShell` variants and their `runtime_shell.rs` handlers. `MoveWindow` is
where the animation disable/dispatch/restore lives, with a comment saying why
the cut has to be silent.

*Done when:* `eww-state dump` shows the machine and a hand-driven
`hyprctl dispatch` equivalent does the same thing.

### 4. Wiring

Route `EventProbe::Spaces`, `EventUi::WindowCloseRequested` and the job
completions through `StateBar`; add the `StateId`; add the IPC verb
(`eww-state window close`) alongside the existing ones in `ipc.rs`.

*Done when:* a scenario test in `crates/core/tests/scenarios.rs` replays the
bugs the old setup had — the echo bounce, a close inside a Space, a window
closing mid-move — with no compositor and no sleeping.

### 5. Cutover

Only now, and outside this repo:

- delete the `__mac_spaces_init` block from `hyprland.lua`, keeping the
  keybinds, the 3-finger gesture and the animation config
- delete `scripts/prune-spaces.sh` and `scripts/mac-fullscreen.sh`
- point `SUPER+Q` and the red traffic-light button at `eww-state window close`
- start the daemon from `hyprland.start` — it is not started there today; the
  live session still runs plain `eww daemon; eww open bar; eww open dock`, so
  none of this reaches the desktop until that line changes
- `rm -rf $XDG_RUNTIME_DIR/mac-fs`

Then check by hand, since none of it is testable: enter and leave fullscreen by
all three routes (green button, double-click, an app's own F11), close from
inside a Space, swipe between Spaces, and `hyprctl reload` mid-Space — the
guard against duplicate handlers exists because reload re-runs the file, and the
daemon should not care at all.

This stage changes the user's live desktop. Get it confirmed before running it.

## Not in scope

**Multi-monitor.** There is one monitor (`eDP-1`) and the design assumes it.
Worth knowing that the old `sync-bars.sh` read `hyprctl monitors -j | jq
'.[0].reserved'` — hardcoded to the first monitor — so the shell version was
already wrong for a second screen. Do not carry that assumption into the machine
where it is invisible: `Client` should carry the monitor even if nothing reads
it yet, and if a rule needs one, make it explicit rather than implicitly zero.

**Tiling or workspace management generally.** This machine moves fullscreen
windows between workspaces. It is not a window manager.
