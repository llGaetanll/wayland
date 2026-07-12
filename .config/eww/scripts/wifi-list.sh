#!/usr/bin/env bash
# Emit the Wi-Fi state as JSON for the eww dropdown:
#   {"enabled":bool,"networks":[{ssid,ssid_b64,signal,icon,active,secure}],"list_h":px}
#
# ssid_b64 is the base64 of the SSID so eww can pass it into an onclick shell
# command with zero quoting/injection risk. list_h sizes the scroll area to the
# number of rows (capped) so the panel hugs its content like a macOS menu.
set -uo pipefail

ICONS="$HOME/.config/eww/icons"

sig_icon() { # signal% -> symbolic icon basename
    local s=$1
    if   [ "$s" -ge 80 ]; then echo nm-signal-100-symbolic
    elif [ "$s" -ge 55 ]; then echo nm-signal-75-symbolic
    elif [ "$s" -ge 30 ]; then echo nm-signal-50-symbolic
    elif [ "$s" -ge 5  ]; then echo nm-signal-25-symbolic
    else                       echo nm-signal-0-symbolic
    fi
}

enabled=false
[ "$(nmcli -t -f WIFI radio 2>/dev/null)" = enabled ] && enabled=true

unescape() { local s=$1; s=${s//\\:/:}; s=${s//\\\\/\\}; printf '%s' "$s"; }

rows=()
connected=""
if [ "$enabled" = true ]; then
    # Fields: IN-USE:SIGNAL:SECURITY:SSID. SSID is last, so take everything after
    # the 3rd colon (handles ':' inside SSIDs); nmcli escapes '\:' and '\\'.
    data=$(nmcli -t -f IN-USE,SIGNAL,SECURITY,SSID device wifi list 2>/dev/null)

    # Pass 1: the connected SSID, from the '*' marker. Do this across ALL BSSes,
    # because a network can appear multiple times (one per band/AP) and the
    # active BSS may not be the strongest one that survives dedupe below.
    while IFS= read -r line; do
        [ "${line%%:*}" = "*" ] || continue
        rest=${line#*:}; rest=${rest#*:}; connected=$(unescape "${rest#*:}"); break
    done <<< "$data"

    # Pass 2: one row per SSID (strongest first), active = matches connected.
    declare -A seen=()
    while IFS= read -r line; do
        rest=${line#*:}
        signal=${rest%%:*}; rest=${rest#*:}
        security=${rest%%:*}; ssid=$(unescape "${rest#*:}")
        [ -z "$ssid" ] && continue
        [ -n "${seen[$ssid]:-}" ] && continue    # dedupe, keep strongest (first)
        seen[$ssid]=1
        [[ "$signal" =~ ^[0-9]+$ ]] || signal=0
        active=false; [ "$ssid" = "$connected" ] && active=true
        secure=true;  [ -z "$security" ] && secure=false
        b64=$(printf '%s' "$ssid" | base64 -w0)
        rows+=("$(jq -nc \
            --arg ssid "$ssid" --arg b64 "$b64" --argjson signal "$signal" \
            --arg icon "$ICONS/$(sig_icon "$signal").svg" \
            --argjson active "$active" --argjson secure "$secure" \
            '{ssid:$ssid,ssid_b64:$b64,signal:$signal,icon:$icon,active:$active,secure:$secure}')")
    done <<< "$data"
fi

# Scroll height: ~40px per row, clamped so the panel neither collapses nor runs
# off-screen.
n=${#rows[@]}
h=$(( n * 40 )); [ "$h" -gt 300 ] && h=300; [ "$h" -lt 40 ] && h=40

if [ "$n" -eq 0 ]; then
    jq -nc --argjson enabled "$enabled" --argjson h "$h" --arg connected "$connected" \
        '{enabled:$enabled,networks:[],list_h:$h,connected:$connected}'
else
    printf '%s\n' "${rows[@]}" | jq -sc --argjson enabled "$enabled" --argjson h "$h" --arg connected "$connected" \
        '{enabled:$enabled,networks:.,list_h:$h,connected:$connected}'
fi
