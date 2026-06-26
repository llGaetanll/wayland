#!/bin/sh
# Load and style hyprbars (mac-style title bars).
#
# hyprpm-enabled plugins are NOT auto-loaded at login, and the Lua config is
# parsed before any plugin loads, so we can't touch hl.plugin.hyprbars at parse
# time. Instead this runs from hyprland.start: load the plugin, wait for it,
# then apply config + buttons via `hyprctl eval`. Runs once per login, so the
# (reset-less) add_button calls don't accumulate duplicates.

hyprpm reload -n

# Wait up to ~10s for the plugin to register.
for _ in $(seq 1 50); do
    hyprctl plugins list | grep -q hyprbars && break
    sleep 0.2
done

hyprctl eval 'hl.config({ plugin = { hyprbars = {
    bar_height = 28,
    bar_color = "rgb(2e2e2e)",
    bar_text_size = 11,
    bar_text_font = "Inter",
    bar_text_align = "center",
    bar_buttons_alignment = "left",
    bar_padding = 10,
    bar_button_padding = 8,
    icon_on_hover = true,
    on_double_click = "hyprctl dispatch fullscreen 1",
} } })'

# Traffic-light buttons (left side, mac order): red = close, green = fullscreen.
# Glyphs only show on hover (icon_on_hover), so normally these read as colored dots.
hyprctl eval 'hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(2e2e2e)", size = 11, icon = "×", action = "hyprctl dispatch killactive" })'
hyprctl eval 'hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(2e2e2e)", size = 11, icon = "+", action = "hyprctl dispatch fullscreen 1" })'
