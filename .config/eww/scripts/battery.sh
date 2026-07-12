#!/usr/bin/env bash
# iPhone-style battery pill for the eww top bar. Generates an SVG (rounded body +
# terminal nub + proportional level-fill + centred haloed number) and echoes its
# path; eww shows it via CSS `background-image`.
#
# The filename encodes cap+status, so the path changes exactly when the pill's
# look changes — that's what makes eww/GTK reload the image (a fixed path with
# new bytes would not). Stale variants are pruned each run.
#
# Adapted from the old waybar battery-pill.sh (dropped the waybar RTMIN signal
# and the JSON output; the pill is self-contained so it reads on a light bar).
set -uo pipefail

BAT=/sys/class/power_supply/BAT0
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/eww"
mkdir -p "$CACHE"

cap=$(cat "$BAT/capacity" 2>/dev/null || echo 0)
status=$(cat "$BAT/status" 2>/dev/null || echo Unknown)
[[ "$cap" =~ ^[0-9]+$ ]] || cap=0
(( cap > 100 )) && cap=100

charging=0
case "$status" in Charging|Full) charging=1 ;; esac
if   (( charging ));   then fill="#30D158"
elif (( cap <= 20 ));  then fill="#FF453A"
else                        fill="#FFFFFF"
fi

out="$CACHE/battery-${cap}-${charging}.svg"
# Prune older variants so the cache holds just the current one.
find "$CACHE" -maxdepth 1 -name 'battery-*.svg' ! -name "${out##*/}" -delete 2>/dev/null || true

if [ ! -f "$out" ]; then
    fs=12; (( cap >= 100 )) && fs=9
    fw=$(awk "BEGIN{printf \"%.2f\", 18.4*$cap/100}")
    cat > "$out" <<SVG
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 17">
  <rect x="0.5" y="0.5" width="21" height="16" rx="4" fill="#4a4a4c"/>
  <rect x="22" y="5.5" width="2.2" height="6" rx="1.1" fill="#4a4a4c"/>
  <rect x="1.8" y="1.8" width="$fw" height="13.4" rx="2.6" fill="$fill"/>
  <text x="11" y="12.4" font-family="Inter, sans-serif" font-size="$fs"
        font-weight="700" text-anchor="middle"
        fill="#000000" stroke="#ffffff" stroke-width="1.2"
        paint-order="stroke" style="paint-order:stroke">$cap</text>
</svg>
SVG
fi

printf '%s\n' "$out"
