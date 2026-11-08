#!/bin/bash
# Records both sides of a call: your mic on the left channel, everything the
# speakers play on the right. Not bound to a key; run it by hand around a call.
# Everything is keyed off $STATE, so a second `start` cannot leave two ffmpegs
# running and a stale entry from a crash is cleaned up rather than obeyed.

set -u

OUTDIR="${CALL_REC_DIR:-$HOME/Recordings}"
STATE=/tmp/record-call.state
MAX_SECONDS=14400

# Hyprland itself is started without a bus address, so children of the
# compositor inherit none; .zprofile puts a session bus at the well-known path,
# and this falls back to it rather than leaving notify-send to vanish.
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/bus}"

# The synchronous hint makes each toast replace the last instead of stacking.
note() {
    notify-send -u "${3:-normal}" -t "${2:-4000}" \
        -h string:x-canonical-private-synchronous:record-call \
        -h string:x-dunst-stack-tag:record-call \
        "$1"
    echo "$1" >&2
}

# Reads $STATE into pid/file/started. Treats a dead pid as not recording and
# clears the file, so a crashed ffmpeg does not block the next start.
load_state() {
    pid=; file=; started=
    [ -f "$STATE" ] || return 1
    IFS=' ' read -r pid file started < "$STATE"
    if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
        rm -f "$STATE"
        return 1
    fi
    return 0
}

hms() {
    local s=$1
    printf '%d:%02d:%02d' $((s / 3600)) $((s % 3600 / 60)) $((s % 60))
}

case "${1:-}" in
start)
    if load_state; then
        note "🔴 Already recording — $(basename "$file")"
        exit 0
    fi

    # No audio source means ffmpeg attaches to nothing and the mic channel
    # reads back as silence. pactl, not wpctl: wpctl lists the camera under
    # Sources too, so it matches any "is there a source" test. Monitors are
    # sink loopbacks, not inputs.
    if ! pactl list short sources 2>/dev/null | grep -qv '\.monitor'; then
        note "❌ No microphone — no audio source" 5000 critical
        exit 1
    fi

    src=$(pactl get-default-source)
    sink=$(pactl get-default-sink)
    mon="${sink}.monitor"

    # The monitor is resolved once, here. Switching output device mid-call
    # leaves this recording attached to the old one, which is why the toast
    # names the device being captured.
    if ! pactl list short sources 2>/dev/null | grep -q "[[:space:]]${mon}[[:space:]]"; then
        note "❌ No monitor for $sink" 5000 critical
        exit 1
    fi

    mkdir -p "$OUTDIR" || exit 1
    file="$OUTDIR/$(date +%F-%H%M%S).ogg"

    # pan to mono per input, then amerge: you land on the left channel and the
    # far side on the right, so either voice can be isolated on playback.
    # setsid detaches it from this terminal, which may well be closed mid-call.
    setsid ffmpeg -nostdin -hide_banner -loglevel error -y \
        -f pulse -i "$src" \
        -f pulse -i "$mon" \
        -filter_complex "[0:a]pan=mono|c0=c0[m];[1:a]pan=mono|c0=0.5*c0+0.5*c1[t];[m][t]amerge=inputs=2[a]" \
        -map "[a]" -ac 2 -c:a libopus -b:a 48k \
        -t "$MAX_SECONDS" "$file" </dev/null >/dev/null 2>&1 &
    pid=$!
    disown "$pid" 2>/dev/null

    # ffmpeg exits early on a bad device, and a pid written for a dead process
    # would read back as "recording" until the next command noticed.
    sleep 0.6
    if ! kill -0 "$pid" 2>/dev/null; then
        rm -f "$file"
        note "❌ Recorder failed to start" 5000 critical
        exit 1
    fi

    echo "$pid $file $(date +%s)" > "$STATE"
    label=${sink##*HiFi__}
    note "🔴 Recording — ${label%__sink}"
    ;;

stop)
    if ! load_state; then
        note "⚪ Not recording" 3000 low
        exit 0
    fi

    # SIGINT, not SIGKILL: ffmpeg only writes the ogg trailer on a clean
    # shutdown, and a killed file is unseekable for its whole length.
    kill -INT "$pid" 2>/dev/null
    for _ in $(seq 40); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    kill -0 "$pid" 2>/dev/null && kill -9 "$pid" 2>/dev/null

    rm -f "$STATE"
    elapsed=$(( $(date +%s) - started ))

    if [ ! -s "$file" ] || [ "$(stat -c %s "$file")" -lt 4096 ]; then
        rm -f "$file"
        note "❌ Nothing captured — check the devices" 5000 critical
        exit 1
    fi

    note "✅ Saved $(hms "$elapsed") — $(basename "$file") ($(du -h "$file" | cut -f1))" 6000
    echo "$file"
    ;;

status)
    if load_state; then
        note "🔴 Recording $(hms $(( $(date +%s) - started ))) — $(basename "$file")" 4000
    else
        note "⚪ Not recording" 3000 low
    fi
    ;;

*)
    echo "usage: ${0##*/} start|stop|status" >&2
    exit 2
    ;;
esac
