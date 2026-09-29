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

-- macOS-like fullscreen "Spaces" live in eww-state, which watches the event
-- socket and owns where a fullscreen window goes; only the swipe is here.
-- scale is a delta multiplier: lower = more finger travel per switch.
hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace", scale = 0.5 })
