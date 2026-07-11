#!/bin/sh
# Show/hide the top bar (waybar) and the dock (eww) to match the FOCUSED
# workspace: hide them when that workspace holds a window we sent to a fullscreen
# "Space" (macOS hides the menu bar + dock on a fullscreen Space), show otherwise.
#
# We key on the per-window state files written by mac-fullscreen.sh (named after
# the window address), NOT on the window's maximize state — so an ordinary
# double-click-maximize on your desktop keeps the bars, only a Space hides them.
#
# Called from hyprland.lua on window.fullscreen / workspace.active / window.close,
# and directly from mac-fullscreen.sh, so the bars track fullscreening, swiping
# between Spaces, and closing a fullscreen app.
#
# The top bar (waybar) only toggles on SIGUSR1 (no absolute show/hide), so we
# read the monitor's reserved area as the source of truth for "is it shown" and
# it can never drift. The dock (eww) has absolute `eww open/close dock`, driven
# on the same transitions so both stay in lockstep. A lock + short settle
# serialises back-to-back events.

runtime="${XDG_RUNTIME_DIR:-/tmp}/mac-fs"
mkdir -p "$runtime"
exec 9>"$runtime/bars.lock"
flock 9

wsid=$(hyprctl activeworkspace -j | jq -r '.id')
[ -n "$wsid" ] && [ "$wsid" != "null" ] || exit 0

clients=$(hyprctl clients -j)
onws=$(printf '%s' "$clients" | jq -r --argjson w "$wsid" '.[]|select(.workspace.id==$w)|.address')
alladdr=$(printf '%s' "$clients" | jq -r '.[].address')

# Should the bars be hidden? -> a tracked Space window lives on the focused
# workspace. Also prune state files whose window has since closed (self-heal).
want_hidden=false
for f in "$runtime"/0x*; do
    [ -e "$f" ] || continue
    a=${f##*/}
    case "$alladdr" in
        *"$a"*) ;;                       # window still exists
        *) rm -f "$f"; continue ;;       # stale -> prune
    esac
    case "$onws" in
        *"$a"*) want_hidden=true ;;
    esac
done

# Are the bars currently shown? -> the top bar (waybar) is reserving space. (The
# eww dock is non-exclusive and never reserves, so it follows the top bar here.)
shown=$(hyprctl monitors -j | jq '.[0].reserved | map(. > 0) | any')

if [ "$want_hidden" = "true" ] && [ "$shown" = "true" ]; then
    pkill -USR1 -x waybar        # hide top bar
    eww close dock 2>/dev/null   # hide dock
    sleep 0.25                   # let the reserved area settle before the lock frees
elif [ "$want_hidden" = "false" ] && [ "$shown" = "false" ]; then
    pkill -USR1 -x waybar        # show top bar
    eww open dock 2>/dev/null    # show dock
    sleep 0.25
fi
