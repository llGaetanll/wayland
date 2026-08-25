# Plans

Work that is designed but not built. One file per plan, deleted once the work
lands — a plan that survives its own implementation becomes a second, stale
description of the code.

A plan is written for whoever picks it up next, so it says what to build, in
what order, and how each stage proves itself. It does not restate
[STATE_MACHINES.md](../docs/STATE_MACHINES.md) or [STYLE.md](../docs/STYLE.md);
it assumes both and names only where the work brushes against them.

- [spaces.md](spaces.md) — move the macOS-style fullscreen "Spaces" out of
  `hyprland.lua` and its shell scripts and into a state machine.
