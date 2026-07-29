#!/bin/sh
# Show/hide the top bar (eww) and the dock (eww) to match the FOCUSED
# workspace: hide them when that workspace holds a window we sent to a fullscreen
# "Space" (macOS hides the menu bar + dock on a fullscreen Space), show otherwise.
#
# We key on the FOCUSED workspace's live fullscreen state: hide if it holds a
# full-screen window. In this setup every full-screen window is whisked onto its
# own Space by the window.fullscreen handler in hyprland.lua, so that test is
# exactly "a Space is focused" — and, being live state rather than a file the
# handler writes later, it's true the INSTANT you go full-screen, so the bars can
# hide immediately instead of waiting for the debounce/move.
#
# Called from hyprland.lua on window.fullscreen / workspace.active / window.close,
# and directly from mac-fullscreen.sh, so the bars track fullscreening, swiping
# between Spaces, and closing a fullscreen app.
#
# Both bars are now eww windows with absolute `eww open/close`. The top bar is
# :exclusive so it reserves space; we read the monitor's reserved area as the
# source of truth for "is it shown" and it can never drift. Both are driven on
# the same transitions so they stay in lockstep. A lock + short settle
# serialises back-to-back events.

runtime="${XDG_RUNTIME_DIR:-/tmp}/mac-fs"
mkdir -p "$runtime"
exec 9>"$runtime/bars.lock"
flock 9

wsid=$(hyprctl activeworkspace -j | jq -r '.id')
[ -n "$wsid" ] && [ "$wsid" != "null" ] || exit 0

clients=$(hyprctl clients -j)
alladdr=$(printf '%s' "$clients" | jq -r '.[].address')

# Should the bars be hidden? -> the focused workspace holds a full-screen window.
want_hidden=$(printf '%s' "$clients" | jq -r --argjson w "$wsid" 'any(.[]; .workspace.id == $w and .fullscreen != 0)')

# Self-heal: prune origin files whose window has since closed. The bars no longer
# depend on these files (they exist only for the close path in mac-fullscreen.sh),
# but pruning here keeps the dir from accumulating orphans over a long session.
for f in "$runtime"/0x*; do
    [ -e "$f" ] || continue
    a=${f##*/}
    case "$alladdr" in *"$a"*) ;; *) rm -f "$f" ;; esac
done

# Are the bars currently shown? -> the top bar (eww, :exclusive) is reserving
# space. (The eww dock is non-exclusive and never reserves, so it follows here.)
shown=$(hyprctl monitors -j | jq '.[0].reserved | map(. > 0) | any')

if [ "$want_hidden" = "true" ] && [ "$shown" = "true" ]; then
    eww close bar 2>/dev/null    # hide top bar
    eww close dock 2>/dev/null   # hide dock
    sleep 0.25                   # let the reserved area settle before the lock frees
elif [ "$want_hidden" = "false" ] && [ "$shown" = "false" ]; then
    eww open bar 2>/dev/null     # show top bar
    eww open dock 2>/dev/null    # show dock
    sleep 0.25
fi
