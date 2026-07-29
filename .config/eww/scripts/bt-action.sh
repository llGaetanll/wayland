#!/usr/bin/env bash
# Actions for the eww Bluetooth dropdown. MACs arrive base64-encoded (see
# bt-list.sh). Mirrors wifi-action.sh.
#
#   bt-action.sh toggle          # controller power on/off
#   bt-action.sh scan            # timed discovery so new devices appear
#   bt-action.sh connect <b64>   # toggle a device: connected -> disconnect,
#                                #   else pair (if needed) + trust + connect
#   bt-action.sh close           # close the dropdown + backdrop
#
# Unlike Wi-Fi, a device click does NOT close the menu — connecting/pairing can
# take a while and it's nicer to watch the status update in place.
set -uo pipefail

EWW="eww"
LOG="${XDG_CACHE_HOME:-$HOME/.cache}/eww-bt.log"
log() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >>"$LOG"; }
notify() { command -v notify-send >/dev/null 2>&1 && notify-send "Bluetooth" "$1" || true; }
refresh() { $EWW update bt="$(~/.config/eww/scripts/bt-list.sh)" 2>/dev/null; }
close_menu() { $EWW update bt_connecting="" 2>/dev/null; ~/.config/eww/scripts/menu.sh close; }

case "${1:-}" in
  toggle)
    if [ "$(bluetoothctl show | awk '/Powered:/{print $2; exit}')" = yes ]; then
        bluetoothctl power off
    else
        bluetoothctl power on
        bluetoothctl --timeout 10 scan on >/dev/null 2>&1 &
    fi
    refresh ;;

  scan)
    bluetoothctl --timeout 10 scan on >/dev/null 2>&1 & ;;

  connect)
    mac=$(printf '%s' "${2:-}" | base64 -d 2>/dev/null)
    [ -z "$mac" ] && exit 0
    $EWW update bt_connecting="$mac" 2>/dev/null
    if bluetoothctl devices Connected 2>/dev/null | awk '{print $2}' | grep -qxF "$mac"; then
        log "disconnect $mac"; bluetoothctl disconnect "$mac" >>"$LOG" 2>&1
        notify "Disconnected"
    else
        # Every device shown in this menu is already known (it comes from the
        # paired/bonded/trusted/connected lists — see bt-list.sh), so a plain
        # `connect` is all that's needed: BlueZ completes any missing pairing
        # itself (it flips Paired no->yes on connect). We deliberately do NOT
        # start a discovery scan or force an explicit `pair` first — a device
        # that isn't in pairing mode makes `pair` fail, and connecting while a
        # scan is active is unreliable; that combination is what made this hang
        # and never connect. `trust` first so it auto-reconnects next time.
        bluetoothctl trust "$mac" >>"$LOG" 2>&1 || true
        log "connect $mac"
        if bluetoothctl connect "$mac" >>"$LOG" 2>&1; then notify "Connected"; else notify "Failed to connect"; fi
    fi
    $EWW update bt_connecting="" 2>/dev/null
    refresh ;;

  close) close_menu ;;
esac
