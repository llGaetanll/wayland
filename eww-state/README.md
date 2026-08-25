# eww-state

One binary that owns the state of the desktop widgets — the top bar, its
dropdowns, the dock and the screenshot preview — replacing the pile of shell
scripts that each pushed their own state into eww.

## Why

The scripts had no shared state, so every widget kept its own and they
contradicted each other: two dropdowns open at once, a slider snapping back to a
value from before the drag, a bar hidden with the dock still showing, a window
eww believed was closed but was still on screen. Adding a lock file per script
made each one safe by itself and did nothing about the interleaving between
them.

## Shape

```
crates/core     the state machine. No IO of any kind.
crates/daemon   the shell around it: probes, actions, sockets, eww.
```

`core` is a pure function of events. It does not run commands, open sockets,
read files or look at a clock. Facts arrive as `Event`s; requests to change the
world leave as `Action`s. That makes every rule in it testable without a
compositor, and it makes the daemon dumb enough to trust.

Inside `core`, each widget is its own machine with its own state and its own
event alphabet — `widgets/{audio,backlight,wifi,bluetooth,battery,clock,theme,
dock,shot}.rs`. They never call each other. `bar.rs` sits on top, routes events
to the right one, and owns the single fact none of them can own alone: which
dropdown is open. A widget that needs something of the bar (*dismiss me*) raises
a `Req` and the bar decides.

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

## Testing

```
cargo test
```

`crates/core/tests/common` is a fake world: it plays nmcli, wpctl, bluetoothctl
and the compositor, answers the actions the machine dispatches and lets a test
assert on what the user would see. `scenarios.rs` replays the bugs the old setup
had — menu mashing, a drag racing a poll, Print during an open dropdown,
fullscreen toggling, eww restarting — with no processes and no sleeping.

The daemon's own parsing (nmcli escaping, slurp geometry, bluetoothctl listings)
and its eww reconciliation are unit-tested the same way, against recorded output
and a fake eww sink.

## Running

```
eww-state daemon                    # start from the compositor
eww-state menu toggle wifi          # what a widget handler calls
eww-state dump                      # state, windows and poll plan
```

`dock.conf` is the dock: one `id<TAB>command<TAB>icon` line per app, in dock
order. The file *is* the order — dragging an app rewrites it.
