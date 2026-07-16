#!/bin/sh
# Close helper for the macOS-Spaces window management.
#
# Enter/exit fullscreen is NO LONGER handled here — it's driven by the
# window.fullscreen event handler in hyprland.lua (see the "macOS-like fullscreen
# Spaces" section). This script only handles CLOSING, which the event model can't
# do cleanly on its own (closing a window doesn't emit a fullscreen-exit event,
# and returning to the origin Space is multi-step).
#
# Invoked as `mac-fullscreen.sh close` by the red traffic-light button and by
# SUPER+Q. If the focused window is in its own fullscreen Space — i.e. the event
# handler left it an origin state file under $XDG_RUNTIME_DIR/mac-fs/ named after
# its address — we first hop back to the Space it was spawned on, so the now-empty
# Space is left behind for Hyprland to auto-destroy (an empty Space that is still
# focused would otherwise linger). Then we close the window. A normal window is
# just closed.

set -u

runtime="${XDG_RUNTIME_DIR:-/tmp}/mac-fs"

win=$(hyprctl activewindow -j)
addr=$(printf '%s' "$win" | jq -r '.address')
if [ -z "$addr" ] || [ "$addr" = "null" ]; then
    exit 0
fi

statefile="$runtime/$addr"

dispatch() { hyprctl dispatch "$1" >/dev/null 2>&1; }

if [ -f "$statefile" ]; then
    origin=$(cat "$statefile")
    [ -n "$origin" ] || origin=1
    rm -f "$statefile"
    dispatch "hl.dsp.focus({ workspace = '$origin' })"
fi
dispatch "hl.dsp.window.close({ window = 'address:$addr' })"

# Give the bars back now that we're on a desktop Space (window.close also fires
# sync-bars from hyprland.lua, but this is more immediate). Idempotent + locked.
"$(dirname "$0")/sync-bars.sh"
