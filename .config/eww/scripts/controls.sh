#!/usr/bin/env bash
# Volume + brightness helpers for the eww controls dropdown (iOS-style sliders).
#
#   controls.sh get-volume | get-brightness     # print current 0-100 (for polls)
#   controls.sh set-volume <n> | set-brightness <n>   # set from a slider onchange
#   controls.sh refresh                         # push both values into eww now
set -uo pipefail

EWW="eww"
getvol() { wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null | awk '{print int($2*100+0.5)}'; }
getbri() { brightnessctl -m 2>/dev/null | awk -F, '{gsub("%","",$4); print $4; exit}'; }

case "${1:-}" in
  get-volume)     getvol ;;
  get-brightness) getbri ;;
  # -l 1 caps volume at 100%. Brightness floor of 1% avoids a fully black screen.
  set-volume)     wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ "${2:-0}%" ;;
  set-brightness) brightnessctl set "${2:-1}%" >/dev/null 2>&1 ;;
  refresh)        $EWW update vol_pct="$(getvol)" bri_pct="$(getbri)" 2>/dev/null ;;
  *) echo 0 ;;
esac
