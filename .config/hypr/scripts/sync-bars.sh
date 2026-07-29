#!/bin/sh
# Show or hide the top bar and dock to match the focused workspace, the way macOS
# hides them on a fullscreen Space.
#
# Keys on the focused workspace's live fullscreen state. Since hyprland.lua whisks
# every fullscreen window onto its own Space, that test means "a Space is
# focused" — and being live state rather than a file written later, it's true the
# instant you go fullscreen, so the bars hide without waiting for the move.
#
# Called from hyprland.lua on window.fullscreen / workspace.active / window.close
# and directly from mac-fullscreen.sh. The top bar is :exclusive, so the monitor's
# reserved area is a source of truth for "is it shown" that can't drift. A lock
# plus a short settle serialises back-to-back events.

runtime="${XDG_RUNTIME_DIR:-/tmp}/mac-fs"
mkdir -p "$runtime"
exec 9>"$runtime/bars.lock"
flock 9

wsid=$(hyprctl activeworkspace -j | jq -r '.id')
[ -n "$wsid" ] && [ "$wsid" != "null" ] || exit 0

clients=$(hyprctl clients -j)
alladdr=$(printf '%s' "$clients" | jq -r '.[].address')

want_hidden=$(printf '%s' "$clients" | jq -r --argjson w "$wsid" 'any(.[]; .workspace.id == $w and .fullscreen != 0)')

# Prune origin files whose window has since closed. The bars don't depend on
# these (only mac-fullscreen.sh's close path does), but pruning keeps the dir
# from accumulating orphans over a long session.
for f in "$runtime"/0x*; do
    [ -e "$f" ] || continue
    a=${f##*/}
    case "$alladdr" in *"$a"*) ;; *) rm -f "$f" ;; esac
done

# The bar is shown iff it is reserving space. The dock is non-exclusive and never
# reserves, so it just follows.
shown=$(hyprctl monitors -j | jq '.[0].reserved | map(. > 0) | any')

if [ "$want_hidden" = "true" ] && [ "$shown" = "true" ]; then
    eww close bar 2>/dev/null
    eww close dock 2>/dev/null
    sleep 0.25   # let the reserved area settle before the lock frees
elif [ "$want_hidden" = "false" ] && [ "$shown" = "false" ]; then
    eww open bar 2>/dev/null
    eww open dock 2>/dev/null
    sleep 0.25
fi
