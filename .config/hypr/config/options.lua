hl.config({ misc = { force_default_wallpaper = 0, disable_hyprland_logo = true } })

-- no_warps: Hyprland otherwise yanks the cursor to the centre of a newly focused
-- window, which is jarring when an app is sent to its own Space.
hl.config({ cursor = { no_hardware_cursors = true, no_warps = true } })

-- A 1px hairline around every window, the same trick the eww menus use: a
-- translucent white edge rather than a drawn colour, dimmer when unfocused.
-- Edges stay grabbable well beyond it via extend_border_grab_area.
-- gaps_in is half of gaps_out: Hyprland applies gaps_in to each side of a tiled
-- window, so the gutter between two windows is 2 * gaps_in, and 10 makes it come
-- out at the same 20 as the gap to the screen edge.
hl.config({ general = {
    resize_on_border = true,
    extend_border_grab_area = 15,
    border_size = 1,
    gaps_out = 20,
    gaps_in = 10,
    ["col.active_border"] = "rgba(ffffff33)",
    ["col.inactive_border"] = "rgba(ffffff1a)",
} })

-- Blur applies to anything with alpha < 1 (e.g. alacritty). xray = false blurs
-- the wallpaper behind a window rather than other windows.
hl.config({ decoration = {
    rounding = 6,
    blur = { enabled = true, size = 6, passes = 3, new_optimizations = true, xray = false },
} })
