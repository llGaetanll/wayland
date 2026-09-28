-- Everything the compositor has to know about eww lives here. eww-state is the
-- only thing that starts eww, opens or closes its windows, or writes its
-- variables; nothing in this config opens a window itself. That is the whole
-- point of it: two things racing to open the same window is what left the old
-- setup with an inconsistent registry and a bar that would not hide.

require "eww.search"
require "eww.screenshot"

-- Blur eww surfaces (bar, dock, menus). The menus sit on eww's default
-- gtk-layer-shell namespace; bar and dock have their own so the slide rules
-- below can name them, and both have to be listed here to keep their blur.
-- The full-screen menu backdrop uses its own "eww-backdrop" namespace to stay
-- out of this -- blurring a fullscreen surface every frame near-locks the
-- machine.
-- KEEP IN SYNC with :namespace in ~/.config/eww/eww.yuck.
hl.layer_rule({ name = "eww-blur", match = { namespace = "^(gtk-layer-shell|eww-bar|eww-dock)$" }, blur = true, ignore_alpha = 0.2 })

-- Bar and dock slide off the edge they are anchored to when eww-state closes
-- them for a fullscreen Space, and slide back in when it reopens them. Nothing
-- in eww animates: the daemon still just opens and closes the surfaces, and the
-- compositor animates the map/unmap. Directions are explicit rather than
-- inferred from the anchor, so a geometry change cannot silently reverse one.
hl.layer_rule({ name = "eww-bar-slide",  match = { namespace = "^eww-bar$" },  animation = "slide top" })
hl.layer_rule({ name = "eww-dock-slide", match = { namespace = "^eww-dock$" }, animation = "slide bottom" })

-- The menus keep the pop they have always had: the layers leaf in
-- config/animations.lua is what makes layer animation happen at all, and
-- without this it would reach them too.
hl.layer_rule({ name = "eww-menu-no-anim", match = { namespace = "^gtk-layer-shell$" }, no_anim = true })
hl.layer_rule({ name = "eww-backdrop-no-anim", match = { namespace = "^eww-backdrop$" }, no_anim = true })

hl.on("hyprland.start", function()
    hl.exec_cmd("eww-state daemon")
end)
