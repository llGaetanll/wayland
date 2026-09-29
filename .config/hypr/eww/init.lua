-- eww-state is the only thing that starts eww, opens or closes its windows, or
-- writes its variables; nothing here opens a window itself. Two things racing to
-- open the same window is what left the old setup with a bar that would not hide.

require "eww.search"
require "eww.screenshot"

-- Blur eww surfaces. The menus sit on eww's default gtk-layer-shell namespace;
-- bar and dock have their own so the slide rules below can name them. The
-- fullscreen menu backdrop stays out of this -- blurring it every frame
-- near-locks the machine.
-- KEEP IN SYNC with :namespace in ~/.config/eww/eww.yuck.
hl.layer_rule({ name = "eww-blur", match = { namespace = "^(gtk-layer-shell|eww-bar|eww-dock|eww-dock-low)$" }, blur = true, ignore_alpha = 0.2 })

-- Bar and dock slide off their anchored edge when eww-state closes them for a
-- fullscreen Space; the compositor animates the map/unmap, eww animates nothing.
-- Directions are explicit so a geometry change cannot silently reverse one.
hl.layer_rule({ name = "eww-bar-slide",  match = { namespace = "^eww-bar$" },  animation = "slide top" })
hl.layer_rule({ name = "eww-dock-slide", match = { namespace = "^eww-dock$" }, animation = "slide bottom" })
-- The lowered dock is a swap, not an arrival: the plain dock is sliding out at
-- the same moment, and sliding this in behind it reads as the dock bouncing.
hl.layer_rule({ name = "eww-dock-low-no-anim", match = { namespace = "^eww-dock-low$" }, no_anim = true })

-- The menus keep their pop: the layers leaf in config/animations.lua would
-- otherwise reach them too.
hl.layer_rule({ name = "eww-menu-no-anim", match = { namespace = "^gtk-layer-shell$" }, no_anim = true })
hl.layer_rule({ name = "eww-backdrop-no-anim", match = { namespace = "^eww-backdrop$" }, no_anim = true })

hl.on("hyprland.start", function()
    hl.exec_cmd("eww-state daemon")
end)
