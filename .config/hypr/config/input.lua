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
-- The guard does nothing on this version and is kept only as a tripwire: the
-- config body runs twice per reload, but each parse gets a fresh Lua state, so
-- this global is always nil when tested. Hyprland clears its own registration
-- lists between parses, which is what actually stops the gesture doubling.
if not _G.__mac_spaces_init then
    _G.__mac_spaces_init = true
    hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace", scale = 0.5 })
end
