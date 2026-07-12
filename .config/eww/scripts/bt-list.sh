#!/usr/bin/env bash
# Emit Bluetooth state as JSON for the eww dropdown:
#   {"enabled":bool,"devices":[{name,mac,mac_b64,connected}],"list_h":px,"connected":name}
#
# mac_b64 is the base64 of the MAC so eww can pass it into an onclick shell
# command with zero quoting/injection risk (raw `mac` is only for the
# "Connecting…" row comparison, never used in a shell). Mirrors wifi-list.sh.
set -uo pipefail

enabled=false
[ "$(bluetoothctl show 2>/dev/null | awk '/Powered:/{print $2; exit}')" = yes ] && enabled=true

rows=()
connected=""
if [ "$enabled" = true ]; then
    connected_macs=$(bluetoothctl devices Connected 2>/dev/null | awk '{print $2}')
    declare -A seen=()
    # "My Devices" = union of paired/bonded/trusted/connected, deduped by MAC —
    # devices you have a relationship with, not the random ones a scan discovers.
    # (No single filter is reliable: bluetoothctl 5.86 here reports known devices
    # as Trusted, with Paired/Bonded = no.)
    while read -r _ mac name; do
        [ -z "$mac" ] && continue
        [ -n "${seen[$mac]:-}" ] && continue
        seen[$mac]=1
        is_conn=false
        if grep -qxF "$mac" <<<"$connected_macs"; then
            is_conn=true; [ -z "$connected" ] && connected="$name"
        fi
        b64=$(printf '%s' "$mac" | base64 -w0)
        rows+=("$(jq -nc --arg name "$name" --arg mac "$mac" --arg b64 "$b64" \
            --argjson connected "$is_conn" \
            '{name:$name,mac:$mac,mac_b64:$b64,connected:$connected}')")
    done < <( { bluetoothctl devices Paired; bluetoothctl devices Bonded; \
                bluetoothctl devices Trusted; bluetoothctl devices Connected; } 2>/dev/null )
fi

n=${#rows[@]}
h=$(( n * 40 )); [ "$h" -gt 300 ] && h=300; [ "$h" -lt 40 ] && h=40

if [ "$n" -eq 0 ]; then
    jq -nc --argjson enabled "$enabled" --argjson h "$h" --arg connected "$connected" \
        '{enabled:$enabled,devices:[],list_h:$h,connected:$connected}'
else
    printf '%s\n' "${rows[@]}" | jq -sc --argjson enabled "$enabled" --argjson h "$h" --arg connected "$connected" \
        '{enabled:$enabled,devices:.,list_h:$h,connected:$connected}'
fi
