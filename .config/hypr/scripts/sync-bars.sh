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
# Visibility is SHARED: either both the top bar and the dock are shown, or neither
# is. `want_hidden` below is that single decision, and both windows are always
# driven the same way.
#
# They are two separate eww windows though, so we compare each against its own
# ACTUAL open state (from `eww active-windows`) rather than inferring both from one
# proxy like the bar's reserved area. The dock is non-exclusive and reserves
# nothing, so a proxy can't see it; and if the two ever drift apart (daemon
# restart, a manual `eww close`, a failed open) a shared proxy reports one value
# for both and the stuck one can never be corrected. Checking each is self-healing:
# every event pulls them back into agreement. A lock + short settle serialises
# back-to-back events.

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

# Which eww windows are actually open right now? `eww active-windows` prints
# `<id>: <name>` per open window; strip to bare names. If the daemon is down this
# is empty, so the "not open" branch below will (re)open + bootstrap it.
opened=$(eww active-windows 2>/dev/null | sed 's/^[^:]*: //')
is_open() { printf '%s\n' "$opened" | grep -qx "$1"; }

# Drive each window independently to the desired state; only touch the ones that
# are actually out of place, so a steady desktop/Space is a cheap no-op.
changed=0
if [ "$want_hidden" = "true" ]; then
    is_open bar  && { eww close bar  2>/dev/null; changed=1; }   # hide top bar
    is_open dock && { eww close dock 2>/dev/null; changed=1; }   # hide dock
else
    is_open bar  || { eww open bar  2>/dev/null; changed=1; }    # show top bar
    is_open dock || { eww open dock 2>/dev/null; changed=1; }    # show dock
fi

# Let the reserved area (top bar is :exclusive) settle before the lock frees, so
# back-to-back events don't race the compositor's relayout.
[ "$changed" = 1 ] && sleep 0.25
exit 0
