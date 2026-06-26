#!/usr/bin/env bash
# Wi-Fi menu for waybar: lists networks via nmcli, connects via a fuzzel popup.
set -euo pipefail

MENU=(fuzzel --dmenu --prompt "Wi-Fi: ")
SEP="  —  " # separator between SSID and signal in the menu line

notify() { command -v notify-send >/dev/null 2>&1 && notify-send "Wi-Fi" "$1" || true; }

# If the radio is off, the only useful action is turning it on.
if [ "$(nmcli -t -f WIFI radio)" != "enabled" ]; then
    choice=$(printf 'Turn Wi-Fi on\n' | "${MENU[@]}") || exit 0
    [ -n "$choice" ] && nmcli radio wifi on && notify "Radio enabled"
    exit 0
fi

# Ask for a fresh scan (best-effort; ignore "scanning too soon" errors).
nmcli device wifi rescan 2>/dev/null || true

# One line per network: "SSID  —  72% WPA2". nmcli lists strongest first.
networks=$(nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID device wifi list \
    | awk -F: -v sep="$SEP" '
        $4 != "" {
            mark = ($1 == "*") ? "● " : "  "
            sec  = ($3 == "") ? "open" : $3
            printf "%s%s%s%s%% %s\n", mark, $4, sep, $2, sec
        }')

choice=$(printf '%s\nRescan\nTurn Wi-Fi off\n' "$networks" | "${MENU[@]}") || exit 0
[ -z "$choice" ] && exit 0

case "$choice" in
    "Rescan")        exec "$0" ;;
    "Turn Wi-Fi off") nmcli radio wifi off && notify "Radio disabled"; exit 0 ;;
esac

# Strip the active marker and the "  —  signal%" suffix back to the bare SSID.
ssid="${choice#● }"
ssid="${ssid%%"$SEP"*}"
[ -z "$ssid" ] && exit 0

# Known connection: just bring it up. Otherwise try open, then prompt for a key.
if nmcli -t -f NAME connection show | grep -qxF "$ssid"; then
    nmcli connection up id "$ssid" && notify "Connected to $ssid"
elif nmcli device wifi connect "$ssid" 2>/dev/null; then
    notify "Connected to $ssid"
else
    pass=$(printf '' | fuzzel --dmenu --password --prompt "Password for $ssid: ") || exit 0
    [ -z "$pass" ] && exit 0
    if nmcli device wifi connect "$ssid" password "$pass"; then
        notify "Connected to $ssid"
    else
        notify "Failed to connect to $ssid"
    fi
fi
