#!/usr/bin/env bash
# Act on the current staged screenshot, then dismiss the eww shot menu.
#
#   shot-action.sh <copy|save|close>
#
#   copy    → put the PNG on the Wayland clipboard (wl-copy)
#   save    → write it to ~/Pictures/Screenshots/ with a dated name
#   discard → delete the staged shot and dismiss
#   close   → just dismiss (used by the click-out backdrop)
#
# The staged file path lives in the eww var `shot_path` (set by screenshot.sh).
set -uo pipefail

EWW="eww"
file="$($EWW get shot_path 2>/dev/null)"

case "${1:-close}" in
  copy)
    [ -f "$file" ] && wl-copy --type image/png < "$file"
    ;;
  save)
    dir="$HOME/Pictures/Screenshots"
    mkdir -p "$dir"
    dest="$dir/Screenshot $(date '+%Y-%m-%d %H.%M.%S').png"
    [ -f "$file" ] && cp -- "$file" "$dest"
    ;;
  discard)
    [ -f "$file" ] && rm -f -- "$file"
    ;;
  close) ;;
esac

$EWW update "shot_open=false"
$EWW close shot-menu shot-backdrop 2>/dev/null
