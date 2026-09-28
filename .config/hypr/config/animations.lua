hl.curve("smooth", { type = "bezier", points = {{0.25, 0.1}, {0.25, 1.0}} })

hl.config({ animations = { enabled = true } })

hl.animation({ leaf = "windows",    enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "windowsIn",  enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
hl.animation({ leaf = "fade",       enabled = true, speed = 3, bezier = "smooth" })

-- Layer surfaces are not animated by default (the leaf ships at speed 0). This
-- is what gives the bar and dock their slide; which way each one goes is the
-- per-namespace rules in eww/init.lua. Slower than the windows leaf on purpose:
-- macOS takes about 0.4s to hide the menu bar and Dock, and at speed 3 the slab
-- is gone before the eye follows it.
hl.animation({ leaf = "layers",    enabled = true, speed = 4, bezier = "smooth", style = "slide" })
-- The in/out leaves are set explicitly rather than left to inherit: they each
-- carry their own speed, and `hyprctl animations` reports them still at 0.
hl.animation({ leaf = "layersIn",  enabled = true, speed = 4, bezier = "smooth", style = "slide" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 4, bezier = "smooth", style = "slide" })
