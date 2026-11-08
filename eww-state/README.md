# eww-state

One binary that owns the state of the desktop — the top bar, its dropdowns, the
dock, the screenshot preview, and which workspace a fullscreen window lives on —
replacing the pile of shell scripts that each pushed their own state into eww.

## Why

The scripts had no shared state, so every widget kept its own and they
contradicted each other: two dropdowns open at once, a slider snapping back to a
value from before the drag, a bar hidden with the dock still showing, a window
eww believed was closed but was still on screen. Adding a lock file per script
made each one safe by itself and did nothing about the interleaving between
them.

Where there was no state to share there was a timer instead. The Spaces logic
debounced the compositor for 140ms, waited 250ms for an animation and restored
another after 100ms, because it had no way to tell an event it had caused from
one the user had. All three are gone: see
[`module-spaces`](crates/module-spaces/src/state.rs).

## Shape

```
crates/api-core     the floor every module stands on: an id, a clock reading,
                    the shape of the eww variables. No IO, no dependencies.
crates/module-*     one widget each: its state machine, its event alphabet,
                    and the commands that serve it.
crates/module-bar   the root machine. Owns the modules and routes between them.
crates/daemon       the loop, the sockets, the timers, and making eww match.
crates/util-*       the `eval!` test macro, and running other programs.
```

A module is a pure function of events. It does not run commands, open sockets,
read files or look at a clock — not even to tell the time, which arrives once a
second as `EventClock::Tick`. Facts arrive as `Event`s; requests to change the
world leave as `Action`s. That makes every rule testable without a compositor,
and it makes the daemon dumb enough to trust.

Each module owns its own state and its own alphabet, and names no other module —
`module-{audio,backlight,wifi,bluetooth,battery,date,theme,dock,shot,spaces}`.
`module-bar` sits on top, routes each event to the one module it belongs to,
re-labels that module's actions on the way out, and owns the single fact none of
them can own alone: which dropdown is open. A module that needs something of the
bar (*dismiss me*) raises a `Req` and the bar decides.

Inside a crate the files are named by kind, so a directory listing says what
each is: `state.rs` decides, `event.rs` and `action.rs` are the vocabulary,
`model.rs` is what they carry, `probe.rs` samples and `perform.rs` acts. The
rules behind all of that are in [docs/STATE_MACHINES.md](docs/STATE_MACHINES.md).

Adding a widget is a new `module-*` crate, one variant in the bar's `Event`, one
in its `Action`, and one line in the routing. It is not a change to any other
module.

Three things are **derived** from state rather than stored beside it, which is
where most of the old bugs lived:

| derived | replaces |
| --- | --- |
| `bar.vars()` — every eww variable | each script writing its own vars |
| `bar.windows()` — what should be on screen | each script opening its own windows |
| `bar.polls()` — how often to sample what | `defpoll` intervals fixed in the yuck |

The daemon reconciles all three after every event: it diffs vars against what it
last published, diffs windows against what it has open, and retunes the probe
intervals. Nothing can drift, because nothing is remembered twice.

## Invariants

- **One dropdown at most.** `Option<MenuId>`, not a boolean per menu. Two-open
  is not a representable state.
- **One event at a time.** A single thread touches the machine; probes, actions
  and clicks all queue on one channel. No two handlers interleave.
- **Optimistic writes win over stale reads.** A poll that was already in flight
  when you moved a slider or flipped a switch cannot undo it.
- **Windows are closed before they are opened.** Switching menus never has two
  panels up, and eww is never asked to open a window it already has (which it
  does not deduplicate — the old orphaned-surface bug).
- **Nothing from the network is interpreted.** SSIDs and MACs travel base64
  encoded between the widget and the daemon.
- **An event we caused is not an event the user caused.** Moving a window makes
  the compositor re-emit the fullscreen it was moved for; the module knows the
  move is its own and does not answer its own echo.
- **eww holding our variables is checked, not assumed.** The socket's inode
  names the server, but `eww reload` empties every variable on the same inode.
  The daemon stamps a generation into a var of eww's own and the probe reads it
  back; a table not carrying the current stamp is published again from scratch.
- **No job stays in flight forever.** A command that never returns — a killed
  daemon, a device that wandered off — is given up on after a timeout, so a
  switch cannot stay stuck mid-flip or a row read "Connecting…" all session.

## Testing

```
cargo test
```

`crates/module-bar/tests/common` is a fake world: it plays nmcli, wpctl,
bluetoothctl and the compositor, answers the actions the machine dispatches and
lets a test assert on what the user would see. `scenarios.rs` replays the bugs
the old setup had — menu mashing, a drag racing a poll, Print during an open
dropdown, a fullscreen window bouncing between workspaces, eww restarting, eww
reloading out from under us — with no processes and no sleeping. Its clock is a field the test moves, so a
five-minute timeout is tested instantly.

Each module also has its own tests, which know nothing of the bar: they ask the
same questions the bar asks and nothing more.

Machine tests drive the state machine through `eval!`, where `<` is an event in
and `>` an action expected out:

```rust
eval!(&mut state,
    < EventWifi::MenuOpened,
    > ActionWifi::Scan,
    > ActionWifi::Sample,
);
```

The daemon's own parsing (nmcli escaping, slurp geometry, bluetoothctl listings)
and its eww reconciliation are unit-tested the same way, against recorded output
and a fake eww sink.

## Running

```
eww-state daemon                    # start from the compositor
eww-state menu toggle wifi          # what a widget handler calls
eww-state window close              # what SUPER+Q is bound to
eww-state dump                      # state, windows and poll plan
```

`dock.conf` is the dock: one `id<TAB>command<TAB>icon` line per app, in dock
order. The file *is* the order — dragging an app rewrites it.
