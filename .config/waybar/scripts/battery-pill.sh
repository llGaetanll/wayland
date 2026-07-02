#!/usr/bin/env bash
# iPhone-style battery pill for waybar.
#
# Waybar has no way to draw a proportional-fill battery with the % *inside* it,
# so we generate an SVG (rounded body + terminal nub + level-fill + centred
# number) and rasterise it to PNG with rsvg-convert. An `image` module displays
# the PNG; we poke waybar with RTMIN+8 after each render so it reloads live.
#
# This script IS the exec of a hidden `custom/batgen` module: it prints JSON so
# hovering the battery area shows a tooltip. The number stays legible on the
# black bar via a white halo (paint-order:stroke) around the black digits.
set -uo pipefail

BAT=/sys/class/power_supply/BAT0
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/waybar"
PNG="$CACHE/battery.png"
mkdir -p "$CACHE"

cap=$(cat "$BAT/capacity" 2>/dev/null || echo 0)
status=$(cat "$BAT/status" 2>/dev/null || echo Unknown)
[[ "$cap" =~ ^[0-9]+$ ]] || cap=0
(( cap > 100 )) && cap=100

# Fill colour by state (iOS-ish): green while charging/full, red when low.
charging=0
case "$status" in Charging|Full) charging=1 ;; esac
if (( charging )); then
    fill="#30D158"
elif (( cap <= 20 )); then
    fill="#FF453A"
else
    fill="#FFFFFF"
fi

# Chunky viewBox "0 0 25 17" (no outline): a solid dark battery body shows the
# empty portion, the state colour is the proportional fill, and a big haloed
# number reads over both. Body inner track is 18.4 wide. "100" needs a smaller
# font to fit three digits.
fs=12
(( cap >= 100 )) && fs=9
fw=$(awk "BEGIN{printf \"%.2f\", 18.4*$cap/100}")

svg='<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 17">
  <rect x="0.5" y="0.5" width="21" height="16" rx="4" fill="#4a4a4c"/>
  <rect x="22" y="5.5" width="2.2" height="6" rx="1.1" fill="#4a4a4c"/>
  <rect x="1.8" y="1.8" width="'"$fw"'" height="13.4" rx="2.6" fill="'"$fill"'"/>
  <text x="11" y="12.4" font-family="Inter, sans-serif" font-size="'"$fs"'"
        font-weight="700" text-anchor="middle"
        fill="#000000" stroke="#ffffff" stroke-width="1.2"
        paint-order="stroke" style="paint-order:stroke">'"$cap"'</text>
</svg>'

printf '%s' "$svg" | rsvg-convert -w 100 -h 68 -o "$PNG" - 2>/dev/null

# Tell the image module to reload the freshly-rendered PNG.
pkill -RTMIN+8 waybar 2>/dev/null || true

printf '{"text":"","tooltip":"Battery %s%% — %s","class":"batgen"}\n' "$cap" "$status"
