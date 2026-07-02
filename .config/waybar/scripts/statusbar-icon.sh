#!/usr/bin/env bash
# Render a WhiteSur symbolic status icon (recoloured white) to a PNG that a
# waybar `image` module displays. One call handles one module: it reads the
# current state, picks the matching SVG, rasterises it, then signals waybar so
# the image reloads live. It also prints JSON so it can double as the exec of a
# hidden custom/<module> "generator" module (kept invisible via empty text).
#
# Usage: statusbar-icon.sh <volume|brightness|network|bluetooth>
#
# WhiteSur symbolic SVGs are a single colour (#363636) with levels encoded via
# per-path opacity, so a straight colour swap to white keeps the greyed steps.
set -uo pipefail

mod="${1:-}"
ICONS="$HOME/.config/waybar/icons"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/waybar"
mkdir -p "$CACHE"

# recolour symbolic svg -> white, rasterise to cache png (rendered 2x for crispness)
render() { # $1 svg basename  $2 output basename
    sed 's/#363636/#ffffff/g' "$ICONS/$1.svg" \
        | rsvg-convert -w 40 -h 40 -o "$CACHE/$2.png" - 2>/dev/null
}

case "$mod" in
  volume)
    read -r vol muted < <(wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null \
        | awk '{printf "%d %d", int($2*100), ($0 ~ /MUTED/)?1:0}')
    vol=${vol:-0}; muted=${muted:-0}
    if   [ "$muted" = 1 ];   then icon=audio-volume-muted-symbolic;  tip="Muted"
    elif [ "$vol" -le 33 ];  then icon=audio-volume-low-symbolic;    tip="Volume ${vol}%"
    elif [ "$vol" -le 66 ];  then icon=audio-volume-medium-symbolic; tip="Volume ${vol}%"
    else                          icon=audio-volume-high-symbolic;   tip="Volume ${vol}%"
    fi
    render "$icon" volume
    pkill -RTMIN+9 waybar 2>/dev/null || true
    printf '{"text":"","tooltip":"%s"}\n' "$tip"
    ;;

  brightness)
    p=$(brightnessctl -m 2>/dev/null | awk -F, '{gsub("%","",$4); print $4; exit}')
    p=${p:-0}
    if   [ "$p" -le 1 ];   then icon=display-brightness-off-symbolic
    elif [ "$p" -le 33 ];  then icon=display-brightness-low-symbolic
    elif [ "$p" -le 66 ];  then icon=display-brightness-medium-symbolic
    else                        icon=display-brightness-symbolic
    fi
    render "$icon" brightness
    pkill -RTMIN+10 waybar 2>/dev/null || true
    printf '{"text":"","tooltip":"Brightness %s%%"}\n' "$p"
    ;;

  network)
    if [ "$(nmcli -t -f WIFI radio 2>/dev/null)" != "enabled" ]; then
        icon=network-wireless-disabled-symbolic; tip="Wi-Fi off"
    else
        line=$(nmcli -t -f ACTIVE,SIGNAL,SSID device wifi 2>/dev/null | awk -F: '$1=="yes"{print; exit}')
        if [ -n "$line" ]; then
            sig=$(printf '%s' "$line" | cut -d: -f2); sig=${sig:-0}
            ssid=$(printf '%s' "$line" | cut -d: -f3-)
            if   [ "$sig" -ge 80 ]; then icon=nm-signal-100-symbolic
            elif [ "$sig" -ge 55 ]; then icon=nm-signal-75-symbolic
            elif [ "$sig" -ge 30 ]; then icon=nm-signal-50-symbolic
            elif [ "$sig" -ge 5  ]; then icon=nm-signal-25-symbolic
            else                         icon=nm-signal-0-symbolic
            fi
            tip="${ssid}  (${sig}%)"
        elif nmcli -t -f STATE,TYPE device 2>/dev/null | grep -q '^connected:ethernet'; then
            icon=network-wired-symbolic; tip="Wired"
        else
            icon=network-wireless-offline-symbolic; tip="Not connected"
        fi
    fi
    render "$icon" network
    pkill -RTMIN+11 waybar 2>/dev/null || true
    printf '{"text":"","tooltip":"%s"}\n' "$tip"
    ;;

  bluetooth)
    powered=$(bluetoothctl show 2>/dev/null | awk '/Powered:/{print $2; exit}')
    conns=$(bluetoothctl devices Connected 2>/dev/null | grep -c .)
    if   [ "$powered" != "yes" ];  then icon=bluetooth-disabled-symbolic; tip="Bluetooth off"
    elif [ "${conns:-0}" -gt 0 ];  then icon=bluetooth-paired-symbolic;   tip="${conns} connected"
    else                                icon=bluetooth-active-symbolic;    tip="Bluetooth on"
    fi
    render "$icon" bluetooth
    pkill -RTMIN+12 waybar 2>/dev/null || true
    printf '{"text":"","tooltip":"%s"}\n' "$tip"
    ;;

  *)
    echo '{"text":"","tooltip":""}'
    ;;
esac
