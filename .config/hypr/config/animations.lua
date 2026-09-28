hl.curve("smooth", { type = "bezier", points = {{0.25, 0.1}, {0.25, 1.0}} })
-- Fast off the mark, long tail: what a window shoved aside by a new neighbour
-- should do, so the reflow reads as instant without ending abruptly.
hl.curve("outQuint", { type = "bezier", points = {{0.16, 1.0}, {0.3, 1.0}} })
-- The same shape pushed about as far as it goes: the distance is gone in the
-- first few hundredths and everything after that is the window creeping the
-- last pixels into place. Flatter than this and the move reads as a teleport,
-- because the coast has no distance left to cover.
hl.curve("outExpo", { type = "bezier", points = {{0.01, 1.0}, {0.03, 1.0}} })
-- Time-reversed outQuint, for playing the open animation backwards on close:
-- a bezier runs in reverse when both control points are mirrored through the
-- centre, so (0.16, 1) and (0.3, 1) become (0.7, 0) and (0.84, 0).
hl.curve("inQuint", { type = "bezier", points = {{0.7, 0.0}, {0.84, 0.0}} })

hl.config({ animations = { enabled = true } })

hl.animation({ leaf = "windows",    enabled = true, speed = 3, bezier = "smooth", style = "popin 100%" })
-- A new window grows a little out of its own centre while it fades in (fadeIn
-- carries the opacity half; popin only scales). 95% is deliberately a hint of
-- growth rather than a visible zoom.
hl.animation({ leaf = "windowsIn",  enabled = true, speed = 3, bezier = "outQuint", style = "popin 95%" })
-- Closing is the opening run backwards: the same 0.3s and the same 95% scale.
-- The curve is outQuint rather than its mirror because Hyprland already runs
-- the popin backwards for the out leaf, so mirroring it here inverted it twice.
hl.animation({ leaf = "windowsOut", enabled = true, speed = 3, bezier = "outQuint", style = "popin 95%" })
-- Speeds are deciseconds: 3 is the 0.3s reflow of the windows already on
-- screen, whether they are being pushed aside by a new window or closing the
-- gap left by one that went away. Both directions take the same shape: almost
-- all of that time is the tail end of the ease.
hl.animation({ leaf = "windowsMove", enabled = true, speed = 3, bezier = "outExpo" })
hl.animation({ leaf = "fade",       enabled = true, speed = 3, bezier = "smooth" })
hl.animation({ leaf = "fadeIn",     enabled = true, speed = 3, bezier = "outQuint" })
-- Same double-inversion as windowsOut: the out leaf already runs backwards,
-- so the fade needs the un-mirrored curve to be gone by the time the shrink is.
hl.animation({ leaf = "fadeOut",    enabled = true, speed = 3, bezier = "outQuint" })

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
