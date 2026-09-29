hl.curve("smooth", { type = "bezier", points = {{0.25, 0.1}, {0.25, 1.0}} })
hl.curve("outQuint", { type = "bezier", points = {{0.16, 1.0}, {0.3, 1.0}} })
-- About as flat as a bezier goes: distance is gone in the first few hundredths.
-- Flatter than this and a window move reads as a teleport.
hl.curve("outExpo", { type = "bezier", points = {{0.01, 1.0}, {0.03, 1.0}} })
-- Time-reversed outQuint (both control points mirrored through the centre).
hl.curve("inQuint", { type = "bezier", points = {{0.7, 0.0}, {0.84, 0.0}} })

hl.config({ animations = { enabled = true } })

-- Speeds are deciseconds. The out leaves use the un-mirrored curve because
-- Hyprland already plays them backwards; mirroring here inverted them twice.
hl.animation({ leaf = "windows",    enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "windowsIn",  enabled = true, speed = 3, bezier = "outQuint", style = "popin 95%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 3, bezier = "outQuint", style = "popin 95%" })
hl.animation({ leaf = "windowsMove", enabled = true, speed = 3, bezier = "outExpo" })
hl.animation({ leaf = "fade",       enabled = true, speed = 3, bezier = "smooth" })
hl.animation({ leaf = "fadeIn",     enabled = true, speed = 3, bezier = "outQuint" })
hl.animation({ leaf = "fadeOut",    enabled = true, speed = 3, bezier = "outQuint" })

-- Layer surfaces ship at speed 0; this is what gives the bar and dock their
-- slide (directions are the per-namespace rules in eww/init.lua). Slower than
-- windows on purpose, to match macOS hiding the menu bar in about 0.4s.
-- In/out are set explicitly because they carry their own speed and stay at 0.
hl.animation({ leaf = "layers",    enabled = true, speed = 4, bezier = "outQuint", style = "slide" })
hl.animation({ leaf = "layersIn",  enabled = true, speed = 4, bezier = "outQuint", style = "slide" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 4, bezier = "outQuint", style = "slide" })
