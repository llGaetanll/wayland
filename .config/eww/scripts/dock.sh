#!/usr/bin/env bash
# Dock app registry and order, with drag-to-swap support.
#
# Contents live in the `reg_*` registry below. Order is the only mutable state,
# persisted to $ORDER_FILE one app id per line. `list` emits the apps as JSON for
# eww's `dock_apps` var; `swap A B` exchanges two ids and pushes the new list
# into eww so the dock reorders live rather than on the next poll.

set -euo pipefail

ORDER_FILE="$HOME/.config/eww/dock-order"

# To add an app: add an id here (cmd + icon), and to DEFAULT_ORDER if you want a
# specific spot. Unknown ids in the order file are skipped; registry apps missing
# from it are appended.
reg_cmd() {
  case "$1" in
    firefox)   echo "firefox" ;;
    alacritty) echo "alacritty" ;;
    nemo)      echo "nemo" ;;
    discord)   echo "discord" ;;
    signal)    echo "signal-desktop" ;;
    *)         return 1 ;;
  esac
}
reg_icon() {
  case "$1" in
    firefox)   echo "/usr/share/icons/hicolor/128x128/apps/firefox.png" ;;
    alacritty) echo "/usr/share/icons/WhiteSur-light/apps/scalable/Alacritty.svg" ;;
    nemo)      echo "/usr/share/icons/WhiteSur-light/apps/scalable/nemo.svg" ;;
    discord)   echo "/usr/share/icons/WhiteSur-light/apps/scalable/discord.svg" ;;
    signal)    echo "/usr/share/icons/hicolor/256x256/apps/signal-desktop.png" ;;
    *)         return 1 ;;
  esac
}
DEFAULT_ORDER=(firefox alacritty nemo discord signal)

# Read the persisted order, drop unknown ids, then append any registry app not
# listed yet, so newly-added apps show up without editing the file.
read_order() {
  local -a order=()
  if [[ -f "$ORDER_FILE" ]]; then
    while IFS= read -r id; do
      [[ -n "$id" ]] && reg_cmd "$id" >/dev/null 2>&1 && order+=("$id")
    done < "$ORDER_FILE"
  fi
  local app seen
  for app in "${DEFAULT_ORDER[@]}"; do
    seen=0
    for id in "${order[@]:-}"; do [[ "$id" == "$app" ]] && seen=1 && break; done
    [[ "$seen" == 0 ]] && order+=("$app")
  done
  printf '%s\n' "${order[@]}"
}

write_order() { printf '%s\n' "$@" > "$ORDER_FILE"; }

cmd_list() {
  local -a order
  mapfile -t order < <(read_order)
  local out="[" first=1 id
  for id in "${order[@]}"; do
    [[ "$first" == 1 ]] || out+=","
    first=0
    out+=$(printf '{"id":"%s","cmd":"%s","icon":"%s"}' "$id" "$(reg_cmd "$id")" "$(reg_icon "$id")")
  done
  out+="]"
  echo "$out"
}

cmd_swap() {
  local a="$1" b="$2"
  [[ "$a" == "$b" ]] && return 0
  local -a order
  mapfile -t order < <(read_order)
  local ia=-1 ib=-1 i
  for i in "${!order[@]}"; do
    [[ "${order[$i]}" == "$a" ]] && ia=$i
    [[ "${order[$i]}" == "$b" ]] && ib=$i
  done
  (( ia < 0 || ib < 0 )) && return 0
  local tmp="${order[$ia]}"
  order[$ia]="${order[$ib]}"
  order[$ib]="$tmp"
  write_order "${order[@]}"
  eww update dock_apps="$(cmd_list)" 2>/dev/null || true
}

case "${1:-list}" in
  list) cmd_list ;;
  swap) cmd_swap "${2:?}" "${3:?}" ;;
  *)    echo "usage: dock.sh {list|swap A B}" >&2; exit 1 ;;
esac
