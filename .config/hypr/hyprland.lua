
local mod = "SUPER"

local terminal = os.getenv("TERMINAL") or "alacritty"
local browser = os. getenv("BROWSER") or "firefox"

-- Basic Keybinds 
hl.bind(mod .. " + Q",      hl.dsp.window.close())
hl.bind(mod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mod .. " + W", hl.dsp.exec_cmd(browser))

-- Remap Caps Lock to Esc
hl.config({
    input = {
	kb_options = "caps:escape"
    },
})

-- Cursor Theming
hl.env("XCURSOR_THEME", "WhiteSur-cursors")
hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_THEME", "WhiteSur-cursors")
hl.env("HYPRCURSOR_SIZE", "24")

-- Set Wallpaper
hl.config({ misc = { force_default_wallpaper = 0, disable_hyprland_logo = true } })
hl.on("hyprland.start", function() hl.exec_cmd("hyprpaper") end)

-- Top Bar
hl.on("hyprland.start", function() hl.exec_cmd("waybar") end)

-- Animation Curves
hl.curve("smooth", { type = "bezier", points = {{0.25, 0.1}, {0.25, 1.0}} })

hl.config({
    animations = {
	enabled = true
    }
})

hl.animation({ leaf = "windows", enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "fade", enabled = true, speed = 3, bezier = "smooth" })

-- Fix Trackpad
hl.config({
    input = {
	touchpad = {
	    tap_to_click = true,
	    natural_scroll = true,
	    -- tap_button_map = "lrm",
	    clickfinger_behavior = true,
	}
    }
})

-- UI Scaling
hl.monitor({output = "eDP-1", mode = "preferred", position = "auto", scale = 1.0 })
