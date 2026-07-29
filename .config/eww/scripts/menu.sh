#!/usr/bin/env bash
# The only writer of the top bar's menu state.
#
#   menu.sh toggle <name>   # <name> already open -> close; else switch to it
#   menu.sh close           # close whatever is open
#
# Scope is the dropdowns hanging off a top-bar status icon. The screenshot
# preview (`shot-menu`) is deliberately not one of them: it's a keybind popup,
# not a bar widget, and adding it here would make a Print press dismiss whatever
# bar menu happened to be open.
#
# The single `menu` var holds the name of the open menu, or "" when none. No
# per-menu `*_open` boolean, because N booleans can encode illegal states (two
# menus open at once) and keeping them consistent is N-squared coordination
# between scripts. With one var the invariant is structural. Every menu window is
# named `<name>-menu` and they share one `menu-backdrop`.
#
# Two rules for callers:
# 1. Nothing else may `eww open`/`close` a menu window or write `menu`, or the
#    var and the real window state drift apart.
# 2. Invoke this detached (`setsid -f`). eww kills a widget's command after
#    :timeout (200ms) and this script makes several IPC round-trips, so inline it
#    can die after `update` but before `close`. That drift is unrecoverable: eww
#    0.5.0 doesn't dedupe opens, so the next click orphans a layer surface that
#    `close` can't reap.
set -uo pipefail

EWW="eww"
S="$HOME/.config/eww/scripts"

# Fresh scan / value pull when a menu opens. Backgrounded so none of it holds up
# the window appearing.
onopen() {
  case "$1" in
    wifi)              "$S/wifi-action.sh" rescan ;;
    bt)                "$S/bt-action.sh" scan ;;
    volume|brightness) "$S/controls.sh" refresh ;;
  esac
}

valid() {
  case "$1" in wifi|bt|volume|brightness) return 0 ;; *) return 1 ;; esac
}

# One global lock, not one per menu: the point is to serialise menus against each
# other, so a wifi click and a bt click must take the same one.
exec 9>"${XDG_RUNTIME_DIR:-/tmp}/eww-menu.lock"
flock 9

cur="$($EWW get menu 2>/dev/null)" || cur=""

close_all() {
  # Close the backdrop even when `cur` is empty: an interrupted run can leave the
  # var under-reporting, and closing an already-closed window is a no-op.
  $EWW update menu="" 2>/dev/null
  if [ -n "$cur" ]; then
    $EWW close "${cur}-menu" menu-backdrop 2>/dev/null
  else
    $EWW close menu-backdrop 2>/dev/null
  fi
}

switch_to() {
  local target="$1"
  [ "$target" = "$cur" ] && return 0
  # Keep an existing backdrop up: it's already correct for the incoming menu, and
  # closing then reopening it flickers.
  if [ -n "$cur" ]; then
    $EWW close "${cur}-menu" 2>/dev/null
    $EWW update menu="$target" 2>/dev/null
    $EWW open "${target}-menu"
  else
    $EWW update menu="$target" 2>/dev/null
    $EWW open-many menu-backdrop "${target}-menu"
  fi
  # 9>&- is load-bearing: otherwise the background job inherits the lock fd and
  # holds the flock until it exits, so clicking bt then any other icon stalls for
  # the ten seconds of `bluetoothctl scan`.
  onopen "$target" >/dev/null 2>&1 9>&- &
}

case "${1:-}" in
  toggle)
    valid "${2:-}" || exit 1
    if [ "$cur" = "$2" ]; then close_all; else switch_to "$2"; fi ;;
  close)
    close_all ;;
  *) exit 1 ;;
esac
