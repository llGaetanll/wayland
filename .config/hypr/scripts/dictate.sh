#!/bin/bash
# Push-to-talk dictation. The Copilot key is bound twice in keybinds/dictate.lua:
# `start` on press, `stop` on release, so recording lasts exactly as long as the
# key is held. Everything is keyed off $PIDFILE, so a missed release or a double
# press cannot leave two arecords running.

set -u

WAV=/tmp/dictate.wav
PIDFILE=/tmp/dictate.pid
STATE=/tmp/dictate.state
MAX_SECONDS=120

# First model found wins; WHISPER_MODEL overrides. whisper-cpp ships no model.
find_model() {
    [ -n "${WHISPER_MODEL:-}" ] && { echo "$WHISPER_MODEL"; return; }
    for m in "$HOME"/.local/share/whisper/ggml-*.bin \
             /usr/share/whisper.cpp/models/ggml-*.bin \
             /usr/share/whisper.cpp-model-*/ggml-*.bin; do
        [ -f "$m" ] && { echo "$m"; return; }
    done
}

# Hyprland itself is started without a bus address, so children of the
# compositor inherit none; .zprofile puts a session bus at the well-known path,
# and this falls back to it rather than leaving notify-send to vanish.
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/bus}"

# The synchronous hint makes each toast replace the last instead of stacking.
note() {
    notify-send -u low -t "${2:-2000}" \
        -h string:x-canonical-private-synchronous:dictate \
        -h string:x-dunst-stack-tag:dictate \
        "$1"
}

case "${1:-}" in
start)
    [ -f "$PIDFILE" ] && exit 0

    # No audio source means pw-record attaches to nothing and leaves a 44-byte
    # header; say so here rather than letting it read back as silence.
    # pactl, not wpctl: wpctl prints a Sources list under Video too, and the
    # camera node there matches any "is there a source" test. Monitors are sink
    # loopbacks, not inputs.
    if ! pactl list short sources 2>/dev/null | grep -qv '\.monitor'; then
        note "❌ No microphone — no audio source" 4000
        exit 1
    fi
    # pw-record, not arecord: ALSA's default device here is whatever PipeWire
    # exposes, and it refuses 16k mono outright. pw-record follows the default
    # source and resamples itself.
    timeout "$MAX_SECONDS" pw-record --rate 16000 --channels 1 --format s16 "$WAV" &
    echo $! > "$PIDFILE"
    echo recording > "$STATE"
    note "🔴 Listening…" $((MAX_SECONDS * 1000))
    ;;

stop)
    [ -f "$PIDFILE" ] || exit 0
    pid=$(cat "$PIDFILE")
    rm -f "$PIDFILE"
    # SIGINT, not SIGKILL: pw-record only writes the final RIFF length when it
    # shuts down cleanly, and whisper rejects a wav with a stub header.
    kill -INT "$pid" 2>/dev/null
    pkill -INT -P "$pid" pw-record 2>/dev/null
    wait "$pid" 2>/dev/null

    # A wav this small is a bare RIFF header: nothing was ever captured.
    if [ ! -s "$WAV" ] || [ "$(stat -c %s "$WAV")" -lt 1024 ]; then
        rm -f "$WAV" "$STATE"
        note "❌ Nothing captured — check the mic" 4000
        exit 1
    fi

    model=$(find_model)
    if [ -z "$model" ]; then
        rm -f "$WAV" "$STATE"
        note "❌ No whisper model found" 4000
        exit 1
    fi

    echo transcribing > "$STATE"
    note "💬 Transcribing…" 30000

    text=$(whisper-cli -m "$model" -f "$WAV" -t "$(nproc)" \
               --no-prints --no-timestamps --language auto 2>/dev/null \
           | tr '\n' ' ' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

    # whisper emits these for silence rather than returning nothing.
    case "$text" in
        "" | "[BLANK_AUDIO]" | "(silence)" | "[ Silence ]") text="" ;;
    esac

    rm -f "$WAV" "$STATE"

    if [ -z "$text" ]; then
        note "❌ No speech detected" 2000
        exit 0
    fi

    note "✅ ${text:0:64}" 1500
    wtype -d 2 -- "$text"
    ;;

*)
    echo "usage: ${0##*/} start|stop" >&2
    exit 2
    ;;
esac
