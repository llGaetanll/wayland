#!/bin/sh
# Close helper for the macOS-Spaces window management, invoked as
# `mac-fullscreen.sh close` by the red traffic-light button and SUPER+Q.
#
# Entering and exiting fullscreen is handled by the window.fullscreen handler in
# hyprland.lua; only closing lives here, since closing emits no fullscreen-exit
# event and returning to the origin Space is multi-step.
#
# If the focused window has an origin state file under $XDG_RUNTIME_DIR/mac-fs/,
# it's in its own Space, so hop back to where it was spawned before closing —
# an emptied Space that is still focused lingers instead of auto-destroying.

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

# Give the bars back immediately; window.close also fires sync-bars from
# hyprland.lua, but later. Idempotent and locked.
"$(dirname "$0")/sync-bars.sh"
