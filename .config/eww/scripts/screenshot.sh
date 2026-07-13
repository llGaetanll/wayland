#!/usr/bin/env bash
# macOS-style region screenshot → eww preview menu.
#
#   screenshot.sh
#
# Flow: slurp gives a crosshair region select; grim captures that region to a
# fresh staging file; then the eww `shot-menu` opens top-right showing the shot
# with Copy / Save actions (shot-action.sh). Bound to Print (see hyprland.lua).
set -uo pipefail

EWW="eww"
STAGE_DIR="${XDG_RUNTIME_DIR:-/tmp}/eww-shots"
mkdir -p "$STAGE_DIR"

# slurp: crosshair region select, themed to the macOS accent (#0a84ff). A cancel
# (Esc / right-click) makes slurp exit non-zero with empty output → abort quietly
# WITHOUT touching the menu, so an accidental trigger leaves nothing behind.
geom=$(slurp -b 00000040 -c 0a84ffff -s 0a84ff26 -w 2 2>/dev/null) || exit 0
[ -n "$geom" ] || exit 0

# Serialise from here on (a second Print press can't race the window mutation),
# and close any menu still open from a previous shot before opening a fresh one —
# eww 0.5.0 does NOT dedupe opens, so re-opening an open window orphans a surface.
exec 9>"$STAGE_DIR/.lock"
flock 9
if [ "$($EWW get shot_open 2>/dev/null)" = true ]; then
  $EWW close shot-menu shot-backdrop 2>/dev/null
fi

# Fresh filename per shot: eww/GTK cache background-images by path, so reusing one
# path would show the PREVIOUS screenshot. A timestamped name busts that cache; we
# also reap older staged shots first so the dir doesn't grow unbounded.
find "$STAGE_DIR" -maxdepth 1 -name '*.png' -delete 2>/dev/null
file="$STAGE_DIR/shot-$(date +%s%N).png"

grim -g "$geom" "$file" || exit 1

# Preview box size: preserve the captured aspect ratio, capped to 320px wide
# (monitor scale is 1.0, so slurp's WxH == pixel WxH). eww renders the shot as a
# rounded box background sized to these — the same icon/background-image idiom used
# by the dock and status bar, so it clips to rounded corners for free.
read -r w h < <(printf '%s' "$geom" | sed -E 's/.* ([0-9]+)x([0-9]+)$/\1 \2/')
[[ "$w" =~ ^[0-9]+$ ]] || w=320
[[ "$h" =~ ^[0-9]+$ ]] || h=200
maxw=320
if [ "$w" -gt "$maxw" ]; then
  pw=$maxw
  ph=$(( h * maxw / w ))
else
  pw=$w
  ph=$h
fi

$EWW update "shot_path=$file" "shot_w=$pw" "shot_h=$ph" "shot_open=true"
$EWW open-many shot-backdrop shot-menu
