#!/usr/bin/env bash
# Toggle an eww status dropdown + its click-catcher backdrop. Reusable across the
# Wi-Fi and Bluetooth menus.
#
#   menu-toggle.sh <wifi|bt>
#
# IMPORTANT: eww 0.5.0 does NOT dedupe window opens — calling `open` on an
# already-open window spawns a SECOND layer surface that eww then loses track of
# (active-windows shows one, but two exist, and close can't reap the orphan).
# So we must never open when already open. The gate var (wifi_open/bt_open) is
# the single source of truth, and a flock serialises concurrent invocations
# (e.g. a rapid double-click) so two opens can't race.
set -uo pipefail

EWW="eww"
S="$HOME/.config/eww/scripts"

case "${1:-wifi}" in
  wifi)     win=wifi-menu;     back=wifi-backdrop;     var=wifi_open;     onopen="$S/wifi-action.sh rescan" ;;
  bt)       win=bt-menu;       back=bt-backdrop;       var=bt_open;       onopen="$S/bt-action.sh scan" ;;
  volume)     win=volume-menu;     back=volume-backdrop;     var=volume_open;     onopen="$S/controls.sh refresh" ;;
  brightness) win=brightness-menu; back=brightness-backdrop; var=brightness_open; onopen="$S/controls.sh refresh" ;;
  *) exit 1 ;;
esac

exec 9>"${XDG_RUNTIME_DIR:-/tmp}/eww-menu-${1:-wifi}.lock"
flock 9

if [ "$($EWW get "$var" 2>/dev/null)" = true ]; then
    $EWW update "$var=false"
    $EWW close "$win" "$back" 2>/dev/null
else
    $EWW update "$var=true"
    $EWW open-many "$back" "$win"
    eval "$onopen" >/dev/null 2>&1 &
fi
