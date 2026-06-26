#!/usr/bin/env bash
# Bluetooth menu for waybar: lists devices via bluetoothctl, (dis)connects via a fuzzel popup.
set -uo pipefail

MENU=(fuzzel --dmenu --prompt "Bluetooth: ")
SEP="  —  " # separator between device name and MAC in the menu line
LOG="${XDG_CACHE_HOME:-$HOME/.cache}/waybar-bt.log"

log() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >>"$LOG"; }
notify() { command -v notify-send >/dev/null 2>&1 && notify-send "Bluetooth" "$1" || true; }

# If the controller is powered off, the only useful action is powering it on.
if [ "$(bluetoothctl show | awk '/Powered:/ {print $2}')" != "yes" ]; then
    choice=$(printf 'Turn Bluetooth on\n' | "${MENU[@]}") || exit 0
    [ -n "$choice" ] && bluetoothctl power on && notify "Powered on"
    exit 0
fi

# Kick off a timed background scan so newly-visible devices show up on the next open.
bluetoothctl --timeout 10 scan on >/dev/null 2>&1 &

connected=$(bluetoothctl devices Connected 2>/dev/null | awk '{print $2}')

# One line per known/visible device: "● Name  —  AA:BB:CC:DD:EE:FF".
devices=$(while read -r _ mac name; do
    [ -z "$mac" ] && continue
    mark="  "
    grep -qxF "$mac" <<<"$connected" && mark="● "
    printf '%s%s%s%s\n' "$mark" "$name" "$SEP" "$mac"
done < <(bluetoothctl devices))

choice=$(printf '%s\nScan again\nTurn Bluetooth off\n' "$devices" | "${MENU[@]}") || exit 0
[ -z "$choice" ] && exit 0

case "$choice" in
    "Scan again")         exec "$0" ;;
    "Turn Bluetooth off") bluetoothctl power off && notify "Powered off"; exit 0 ;;
esac

# The MAC is the stable trailing token after the separator.
mac="${choice##*"$SEP"}"
[ -z "$mac" ] && exit 0
name="${choice%"$SEP"*}"; name="${name#● }"
log "selected name=[$name] mac=[$mac]"

if grep -qxF "$mac" <<<"$connected"; then
    log "disconnecting $mac"
    bluetoothctl disconnect "$mac" >>"$LOG" 2>&1
    notify "Disconnected $name"
    exit 0
fi

# Not connected. Pair first if needed — discovery must stay active for that, so we
# start a fresh scan and give it a moment before pairing an unknown device.
paired=$(bluetoothctl info "$mac" 2>/dev/null | awk '/Paired:/ {print $2}')
log "paired=$paired"

if [ "$paired" != "yes" ]; then
    bluetoothctl --timeout 20 scan on >/dev/null 2>&1 &
    sleep 2
    log "pairing $mac"
    bluetoothctl pair "$mac" >>"$LOG" 2>&1 || log "pair returned non-zero"
fi

bluetoothctl trust "$mac" >>"$LOG" 2>&1 || true
log "connecting $mac"
if bluetoothctl connect "$mac" >>"$LOG" 2>&1; then
    log "connect OK"
    notify "Connected $name"
else
    log "connect FAILED"
    notify "Failed to connect $name"
fi
