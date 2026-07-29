#!/usr/bin/env bash
# Echo the path of the icon matching the current state of one status module, for
# the eww top bar to show via CSS `background-image`. eww polls this and the icon
# reloads whenever the returned path changes.
#
#   status.sh <volume|brightness|network|bluetooth>
set -uo pipefail

ICONS="$HOME/.config/eww/icons"
p() { printf '%s\n' "$ICONS/$1.svg"; }

case "${1:-}" in
  volume)
    read -r vol muted < <(wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null \
        | awk '{printf "%d %d", int($2*100), ($0 ~ /MUTED/)?1:0}')
    vol=${vol:-0}; muted=${muted:-0}
    if   [ "$muted" = 1 ];  then p audio-volume-muted-symbolic
    elif [ "$vol" -le 33 ]; then p audio-volume-low-symbolic
    elif [ "$vol" -le 66 ]; then p audio-volume-medium-symbolic
    else                         p audio-volume-high-symbolic
    fi ;;

  brightness)
    pct=$(brightnessctl -m 2>/dev/null | awk -F, '{gsub("%","",$4); print $4; exit}')
    pct=${pct:-0}
    if   [ "$pct" -le 1 ];  then p display-brightness-off-symbolic
    elif [ "$pct" -le 33 ]; then p display-brightness-low-symbolic
    elif [ "$pct" -le 66 ]; then p display-brightness-medium-symbolic
    else                         p display-brightness-symbolic
    fi ;;

  network)
    if [ "$(nmcli -t -f WIFI radio 2>/dev/null)" != "enabled" ]; then
        p network-wireless-disabled-symbolic
    else
        line=$(nmcli -t -f ACTIVE,SIGNAL,SSID device wifi 2>/dev/null | awk -F: '$1=="yes"{print; exit}')
        if [ -n "$line" ]; then
            sig=$(printf '%s' "$line" | cut -d: -f2); sig=${sig:-0}
            if   [ "$sig" -ge 80 ]; then p nm-signal-100-symbolic
            elif [ "$sig" -ge 55 ]; then p nm-signal-75-symbolic
            elif [ "$sig" -ge 30 ]; then p nm-signal-50-symbolic
            elif [ "$sig" -ge 5  ]; then p nm-signal-25-symbolic
            else                         p nm-signal-0-symbolic
            fi
        elif nmcli -t -f STATE,TYPE device 2>/dev/null | grep -q '^connected:ethernet'; then
            p network-wired-symbolic
        else
            p network-wireless-offline-symbolic
        fi
    fi ;;

  bluetooth)
    powered=$(bluetoothctl show 2>/dev/null | awk '/Powered:/{print $2; exit}')
    conns=$(bluetoothctl devices Connected 2>/dev/null | grep -c .)
    if   [ "$powered" != "yes" ]; then p bluetooth-disabled-symbolic
    elif [ "${conns:-0}" -gt 0 ]; then p bluetooth-paired-symbolic
    else                               p bluetooth-active-symbolic
    fi ;;

  *) p network-wireless-offline-symbolic ;;
esac
