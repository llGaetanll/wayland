#!/usr/bin/env bash
# Actions for the eww Wi-Fi dropdown. SSIDs arrive base64-encoded (see
# wifi-list.sh) so there is no quoting/injection risk from the onclick command.
#
#   wifi-action.sh toggle          # radio on/off
#   wifi-action.sh rescan          # ask for a fresh scan
#   wifi-action.sh connect <b64>   # connect to SSID (known -> up, open -> join,
#                                  #   secured+unknown -> rofi password prompt)
#   wifi-action.sh close           # close the dropdown + backdrop
set -uo pipefail

EWW="eww"
THEME="$HOME/.config/rofi/themes/menu.rasi"
notify() { command -v notify-send >/dev/null 2>&1 && notify-send "Wi-Fi" "$1" || true; }
close_menu() {
    $EWW update wifi_connecting="" 2>/dev/null
    ~/.config/eww/scripts/menu.sh close
}

case "${1:-}" in
  toggle)
    if [ "$(nmcli -t -f WIFI radio)" = enabled ]; then
        nmcli radio wifi off
    else
        nmcli radio wifi on
        nmcli device wifi rescan 2>/dev/null &   # repopulate the list once it's up
    fi
    # Push fresh state immediately so the switch flips sub-second instead of
    # waiting for the next poll tick.
    $EWW update wifi="$(~/.config/eww/scripts/wifi-list.sh)" ;;

  rescan)
    nmcli device wifi rescan 2>/dev/null || true ;;

  connect)
    ssid=$(printf '%s' "${2:-}" | base64 -d 2>/dev/null)
    [ -z "$ssid" ] && exit 0
    $EWW update wifi_connecting="$ssid" 2>/dev/null   # show "Connecting…" on the row
    if nmcli -t -f NAME connection show | grep -qxF "$ssid"; then
        nmcli connection up id "$ssid" && notify "Connected to $ssid"
    elif nmcli device wifi connect "$ssid" 2>/dev/null; then
        notify "Connected to $ssid"
    else
        pass=$(printf '' | rofi -dmenu -password -theme "$THEME" -p "$ssid") || { close_menu; exit 0; }
        [ -z "$pass" ] && { close_menu; exit 0; }
        if nmcli device wifi connect "$ssid" password "$pass"; then notify "Connected to $ssid"
        else notify "Failed to connect to $ssid"; fi
    fi
    close_menu ;;

  close) close_menu ;;
esac
