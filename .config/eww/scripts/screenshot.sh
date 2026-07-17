#!/usr/bin/env bash
# macOS-style region screenshot → eww preview menu.
#
#   screenshot.sh
#
# Flow: grab the WHOLE screen up-front, let slurp pick a region, crop the grab to
# it, then open the eww `shot-menu` top-right with Copy / Save / Discard actions
# (shot-action.sh). Bound to Print / SUPER+SHIFT+S (see hyprland.lua).
set -uo pipefail

EWW="eww"
STAGE_DIR="${XDG_RUNTIME_DIR:-/tmp}/eww-shots"
mkdir -p "$STAGE_DIR"

# Reap older staged shots first so the dir doesn't grow unbounded (both the full
# grabs and cropped shots match *.png; none of this run's files exist yet).
find "$STAGE_DIR" -maxdepth 1 -name '*.png' -delete 2>/dev/null

# Grab the full screen BEFORE slurp draws anything. This is what keeps slurp's
# selection overlay (the blue fill + dim) OUT of the shot: we already hold a clean
# frame by the time slurp's surface exists, and we crop THAT. The naive
# `grim -g "$(slurp)"` races — grim can grab a frame in which the compositor hasn't
# yet torn down slurp's overlay, baking the blue region in.
full="$STAGE_DIR/full-$(date +%s%N).png"
grim "$full" || exit 1

# slurp: crosshair region select, themed to the macOS accent (#0a84ff). A cancel
# (Esc / right-click) → empty output; drop the grab and abort quietly so an
# accidental trigger leaves nothing behind.
geom=$(slurp -b 00000040 -c 0a84ffff -s 0a84ff26 -w 2 2>/dev/null)
if [ -z "$geom" ]; then rm -f "$full"; exit 0; fi

# slurp prints "X,Y WxH"; ImageMagick wants "WxH+X+Y". Parse with pure bash.
xy=${geom%% *}; wh=${geom##* }
x=${xy%,*}; y=${xy#*,}
w=${wh%x*}; h=${wh#*x}
if ! [[ "$x" =~ ^[0-9]+$ && "$y" =~ ^[0-9]+$ && "$w" =~ ^[0-9]+$ && "$h" =~ ^[0-9]+$ ]]; then
  rm -f "$full"; exit 1
fi

# Serialise the eww window mutation from here (a second Print can't race it), and
# close any menu still open from a previous shot before opening a fresh one — eww
# 0.5.0 does NOT dedupe opens, so re-opening an open window orphans a surface.
exec 9>"$STAGE_DIR/.lock"
flock 9
if [ "$($EWW get shot_open 2>/dev/null)" = true ]; then
  $EWW close shot-menu shot-backdrop 2>/dev/null
fi

# Crop the clean full grab to the selection. Fresh filename per shot: eww/GTK cache
# background-images by path, so reusing one path would show the PREVIOUS shot.
# +repage resets the canvas so the PNG's dimensions/offset are the crop, not the
# full screen. Monitor scale is 1.0, so slurp's logical coords == grab pixels.
file="$STAGE_DIR/shot-$(date +%s%N).png"
magick "$full" -crop "${w}x${h}+${x}+${y}" +repage "$file" || { rm -f "$full"; exit 1; }
rm -f "$full"

# Preview box size: preserve the captured aspect ratio, capped to 320px wide. eww
# renders the shot as a rounded box background sized to these — the same
# icon/background-image idiom the dock and status bar use, so it clips corners free.
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
