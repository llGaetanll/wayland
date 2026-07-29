#!/usr/bin/env bash
# The ONLY writer of the TOP BAR's menu state.
#
#   menu.sh toggle <name>   # <name> already open -> close; else switch to it
#   menu.sh close           # close whatever is open
#
# Scope: this owns the dropdowns that hang off a top-bar status icon, and only
# those. The screenshot preview (`shot-menu`) is deliberately NOT one of them —
# it isn't a bar widget, it's a transient popup from a keybind, so it is not
# mutually exclusive with these and keeps its own `shot_open` var + backdrop.
# Don't add it here: doing so would make taking a screenshot dismiss whatever
# bar menu happened to be open, which is not what a Print press should do.
#
# ─── State model ────────────────────────────────────────────────────────────
# A SINGLE eww var, `menu`, holds the name of the open menu, or "" when none.
# There is deliberately no per-menu `*_open` boolean: N booleans can encode
# states that are illegal (two menus open at once) and enforcing the invariant
# across them is N-squared coordination between scripts, which is exactly how
# the wifi+bt-both-open bug happened. With one var the invariant is structural —
# "switch to X" is the only transition, so only one menu can ever be up.
#
# Every menu window is named `<name>-menu` and they all share ONE backdrop
# window (`menu-backdrop`), so there is nothing per-menu to keep in sync.
#
# ─── Two rules that callers must follow ─────────────────────────────────────
# 1. Nothing else may `eww open`/`close` a menu window or write `menu`. Route it
#    through here, or the var and the real window state drift apart.
# 2. Callers invoke this DETACHED (`setsid -f`) from a widget handler. eww kills
#    a widget's command after :timeout (default 200ms) — and this script makes
#    several IPC round-trips, so run inline it can be killed after `update` but
#    before `close`, leaving the var lying about what's on screen. That drift is
#    unrecoverable: eww 0.5.0 does not dedupe opens, so the next click re-opens
#    an already-open window and orphans a layer surface that `close` can't reap.
set -uo pipefail

EWW="eww"
S="$HOME/.config/eww/scripts"

# Work kicked off when a menu opens (fresh scan, pull current values). Runs in
# the background — none of it should hold up the window appearing.
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

# One global lock (not one per menu, as before): the whole point is to serialise
# menus against EACH OTHER, so a wifi click and a bt click must take the same one.
exec 9>"${XDG_RUNTIME_DIR:-/tmp}/eww-menu.lock"
flock 9

cur="$($EWW get menu 2>/dev/null)" || cur=""

close_all() {
  # Close the backdrop unconditionally, even when `cur` is empty: if a previous
  # run was interrupted the var can under-report what's open, and closing an
  # already-closed window is a harmless no-op. This is the self-heal path.
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
  # Close the outgoing menu but KEEP the backdrop up when there is one — it's
  # already correct for the incoming menu, and closing/reopening it flickers.
  if [ -n "$cur" ]; then
    $EWW close "${cur}-menu" 2>/dev/null
    $EWW update menu="$target" 2>/dev/null
    $EWW open "${target}-menu"
  else
    $EWW update menu="$target" 2>/dev/null
    $EWW open-many menu-backdrop "${target}-menu"
  fi
  # 9>&- is load-bearing: without it the background job (and anything IT
  # backgrounds, e.g. the 10s `bluetoothctl scan`) inherits the lock fd and holds
  # the flock until it exits — so clicking bt and then any other icon would stall
  # for ten seconds. Closing the fd in the child releases the lock at OUR exit.
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
