-- Remap Caps Lock to Esc
hl.config({ input = {
    kb_options = "caps:escape",
} })

hl.config({ input = {
    touchpad = {
        tap_to_click = true,
        natural_scroll = true,
        -- tap_button_map = "lrm",
        clickfinger_behavior = true,
    },
} })

-- macOS-like fullscreen "Spaces".
-- A window entering a fullscreen state is whisked onto its own empty workspace
-- ("Space") and comes back when it leaves, and the bars hide while one is
-- focused. None of that is here: eww-state watches the compositor event socket
-- and owns where a fullscreen window lives, so this config keeps only what the
-- compositor itself has to know.
--
-- A 3-finger swipe slides between Spaces. scale is a delta multiplier: lower =
-- more finger travel per switch.
--
-- This used to be wrapped in a `_G.__mac_spaces_init` guard against the gesture
-- registering twice. Measured on 0.56.2 (2026-09-28): the config body does run
-- twice per reload, but each parse gets its own fresh Lua state, so the global
-- was always nil when tested and the guard never did anything. Hyprland clears
-- its own registration lists between parses, and `hyprctl binds` shows no
-- duplicates. If a swipe ever jumps two Spaces again, this is where it starts.
hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace", scale = 0.5 })
