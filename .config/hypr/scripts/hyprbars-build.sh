#!/usr/bin/env bash
# Builds the forked hyprbars (SVG button icons) and loads it into the running
# Hyprland. The fork lives as pristine upstream + one patch file, so an update
# is: move the checkout to the commit upstream pins for this Hyprland, re-apply
# the patch, rebuild.
#
#   --auto     rebuild only if the .so was built against another Hyprland (login path)
#   --update   fetch upstream, move to the pinned commit, re-apply the patch, rebuild
#   --load     (un)load the .so in the running session afterwards
#
# With no flags it just rebuilds the current checkout.

set -uo pipefail

REPO="$HOME/files/github/hyprland-plugins"
PATCH="$HOME/.config/hypr/patches/hyprbars-svg-icons.patch"
SO="$REPO/hyprbars/hyprbars.so"
STAMP="$REPO/hyprbars/.built-against"

AUTO=0
UPDATE=0
LOAD=0
for arg in "$@"; do
    case "$arg" in
        --auto) AUTO=1 ;;
        --update) UPDATE=1 ;;
        --load) LOAD=1 ;;
        *) echo "unknown flag: $arg" >&2; exit 2 ;;
    esac
done

die() { echo "hyprbars-build: $*" >&2; exit 1; }

hyprland_commit() {
    Hyprland --version 2>/dev/null | grep -oP 'commit \K[0-9a-f]{40}' | head -1
}

# upstream pins a plugin commit per Hyprland commit in hyprpm.toml
pinned_plugin_commit() {
    local hl="$1"
    git -C "$REPO" show origin/main:hyprpm.toml 2>/dev/null |
        grep -F "\"$hl\"" | grep -oP '"[0-9a-f]{40}"' | tail -1 | tr -d '"'
}

load_plugin() {
    command -v hyprctl >/dev/null || return 0
    hyprctl plugin unload "$SO" >/dev/null 2>&1
    hyprctl plugin load "$SO" >/dev/null || die "plugin load failed"
    hyprctl reload >/dev/null
    echo "hyprbars-build: loaded $SO"
}

build() {
    make -C "$REPO/hyprbars" clean >/dev/null 2>&1
    make -C "$REPO/hyprbars" all || return 1
    hyprland_commit >"$STAMP"
}

HL="$(hyprland_commit)"
[[ -n "$HL" ]] || die "could not read the Hyprland commit"

if ((AUTO)) && [[ -f "$SO" && "$(cat "$STAMP" 2>/dev/null)" == "$HL" ]]; then
    ((LOAD)) && load_plugin
    exit 0
fi

[[ -d "$REPO/.git" ]] || die "$REPO is not a git checkout"
[[ -f "$PATCH" ]] || die "missing patch $PATCH"

if ((UPDATE)); then
    git -C "$REPO" fetch -q origin || die "fetch failed"

    TARGET="$(pinned_plugin_commit "$HL")"
    if [[ -z "$TARGET" ]]; then
        TARGET="$(git -C "$REPO" rev-parse origin/main)"
        echo "hyprbars-build: no pin for Hyprland $HL, using origin/main ($TARGET)"
    fi

    # the patch is the source of truth; keep a copy of anything else edited here
    if ! git -C "$REPO" diff --quiet && ! git -C "$REPO" diff | diff -q - "$PATCH" >/dev/null; then
        git -C "$REPO" diff >"$REPO/.local-changes.patch"
        echo "hyprbars-build: worktree had other edits, saved to $REPO/.local-changes.patch" >&2
    fi

    git -C "$REPO" checkout -qB macos-buttons || die "could not switch to macos-buttons"
    git -C "$REPO" reset -q --hard "$TARGET" || die "reset to $TARGET failed"
    git -C "$REPO" apply --3way "$PATCH" || die "the patch no longer applies — upstream moved, fix $PATCH by hand"
fi

build || die "build failed (after an upstream bump, try --update)"
((LOAD)) && load_plugin
exit 0
