-- Float every window by default (mac-style stacking, not tiling). The handle is
-- returned so the float/tile keybinds can flip the mode for new windows.
local float_all = hl.window_rule({ name = "float-all", match = { class = ".*" }, float = true })

-- Floating windows with no saved geometry map at a tiny default size.
hl.window_rule({ name = "firefox-size", match = { class = "[Ff]irefox" }, size = "1500 950", center = true })

-- A window filling the screen has no edge to draw: the hairline would trace the
-- screen border and the rounding would cut the wallpaper in at the corners.
-- Matched on state rather than on a class, so it follows the window in and out.
hl.window_rule({ name = "fullscreen-no-chrome", match = { fullscreen = true }, border_size = 0, rounding = 0 })

return { float_all = float_all }
