
local mod = "SUPER"

local terminal = os.getenv("TERMINAL") or "alacritty"
local browser = os. getenv("BROWSER") or "firefox"

-- Close through eww-state: a window in a fullscreen Space has to be left behind
-- before it is closed, or the emptied Space lingers instead of auto-destroying.
hl.bind(mod .. " + Q",      hl.dsp.exec_cmd("eww-state window close"))
hl.bind(mod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mod .. " + W", hl.dsp.exec_cmd(browser))

-- Toggle fullscreen on the focused window (moves it to its own Space, see below).
hl.bind(mod .. " + ALT + F", hl.dsp.window.fullscreen({ mode = "fullscreen" }))

-- Spotlight-style launcher, served by eww-state's `search` module. Replaced
-- rofi on 2026-09-28; the old theme is still in ~/.config/rofi/ for reference.
hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))

-- The launcher's keyboard. eww has no key bindings of any kind, so the only
-- thing that can see a key the text field does not want is the compositor.
-- While this submap is active the four keys below come here and every other
-- key still reaches the entry, which is what lets the user keep typing.
--
-- Escape gives the keyboard back here as well as telling the daemon, so a
-- daemon that has died cannot leave the session stuck in a submap.
-- KEEP IN SYNC with SUBMAP in eww-state (crates/module-search/src/perform.rs),
-- which is what enters and leaves it.
hl.define_submap("search", function()
  hl.bind("escape",         hl.dsp.exec_cmd("eww-state search close"))
  hl.bind("escape",         hl.dsp.submap("reset"))
  hl.bind("up",             hl.dsp.exec_cmd("eww-state search move -1"), { repeating = true })
  hl.bind("down",           hl.dsp.exec_cmd("eww-state search move 1"),  { repeating = true })
  hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))
end)

-- no_anim: the launcher resizes its layer surface on every keystroke as the
-- result list grows and shrinks, and animating that makes the text visibly
-- morph. Same reason rofi had this rule. Costs the open fade too.
hl.layer_rule({ name = "eww-search-blur", match = { namespace = "^eww-search$" }, blur = true, ignore_alpha = 0.2, no_anim = true })

-- Blur eww surfaces (bar, dock, menus). The menus sit on eww's default
-- gtk-layer-shell namespace; bar and dock have their own so the slide rules
-- below can name them, and both have to be listed here to keep their blur.
-- The full-screen menu backdrop uses its own "eww-backdrop" namespace to stay
-- out of this — blurring a fullscreen surface every frame near-locks the
-- machine.
-- KEEP IN SYNC with :namespace in ~/.config/eww/eww.yuck.
hl.layer_rule({ name = "eww-blur", match = { namespace = "^(gtk-layer-shell|eww-bar|eww-dock)$" }, blur = true, ignore_alpha = 0.2 })

-- Bar and dock slide off the edge they are anchored to when eww-state closes
-- them for a fullscreen Space, and slide back in when it reopens them. Nothing
-- in eww animates: the daemon still just opens and closes the surfaces, and
-- the compositor animates the map/unmap. Directions are explicit rather than
-- inferred from the anchor, so a geometry change cannot silently reverse one.
hl.layer_rule({ name = "eww-bar-slide",  match = { namespace = "^eww-bar$" },  animation = "slide top" })
hl.layer_rule({ name = "eww-dock-slide", match = { namespace = "^eww-dock$" }, animation = "slide bottom" })

-- The menus keep the pop they have always had: the layers leaf below is what
-- makes layer animation happen at all, and without this it would reach them too.
hl.layer_rule({ name = "eww-menu-no-anim", match = { namespace = "^gtk-layer-shell$" }, no_anim = true })
hl.layer_rule({ name = "eww-backdrop-no-anim", match = { namespace = "^eww-backdrop$" }, no_anim = true })

-- Volume: SUPER +/-  (locked = works on lockscreen, repeating = holds to ramp)
hl.bind(mod .. " + equal", hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + plus",  hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + minus", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),       { locked = true, repeating = true })

-- Brightness: SUPER + ALT +/-  (needs brightnessctl)
hl.bind(mod .. " + ALT + equal", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + plus",  hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + minus", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"), { locked = true, repeating = true })

-- Laptop function row. The keys arrive as ordinary XF86 keysyms on the
-- hid-sdw:...-consumer-control keyboard, so they are bound by keysym and the
-- physical order of the row does not matter. Everything here is `locked` so it
-- keeps working over the lockscreen.
local media = { locked = true }
local media_ramp = { locked = true, repeating = true }

hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), media_ramp)
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),      media_ramp)
hl.bind("XF86AudioMute",        hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"),     media)

-- The Dell privacy driver already cuts the mic in hardware; this keeps the
-- PipeWire source in step so apps see the same state.
hl.bind("XF86AudioMicMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), media)

hl.bind("XF86MonBrightnessUp",   hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), media_ramp)
hl.bind("XF86MonBrightnessDown", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"), media_ramp)

-- Keyboard backlight is a 3-step LED (0/1/2) that cycles on one key. Writing it
-- needs the `input` group; without that these two are no-ops.
hl.bind("XF86KbdBrightnessUp",   hl.dsp.exec_cmd("brightnessctl -d dell::kbd_backlight set +1"), media)
hl.bind("XF86KbdBrightnessDown", hl.dsp.exec_cmd("brightnessctl -d dell::kbd_backlight set 1-"), media)
hl.bind("XF86KbdLightOnOff",     hl.dsp.exec_cmd("sh -c 'brightnessctl -d dell::kbd_backlight set +1 || brightnessctl -d dell::kbd_backlight set 0'"), media)

-- Playback keys go to whichever player has the MPRIS focus.
hl.bind("XF86AudioPlay",  hl.dsp.exec_cmd("playerctl play-pause"), media)
hl.bind("XF86AudioPause", hl.dsp.exec_cmd("playerctl play-pause"), media)
hl.bind("XF86AudioStop",  hl.dsp.exec_cmd("playerctl stop"),       media)
hl.bind("XF86AudioNext",  hl.dsp.exec_cmd("playerctl next"),       media)
hl.bind("XF86AudioPrev",  hl.dsp.exec_cmd("playerctl previous"),   media)

-- Screenshot: slurp region-select → grim → eww preview menu with Copy/Save.
-- eww-state runs the capture and stages the result; a second press while the
-- selector is up is dropped by its state machine rather than by a lock file.
hl.bind("Print",               hl.dsp.exec_cmd("eww-state shot capture"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("eww-state shot capture"))

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
-- no_warps: Hyprland otherwise yanks the cursor to the centre of a newly focused
-- window, which is jarring when an app is sent to its own Space.
hl.config({ cursor = { no_hardware_cursors = true, no_warps = true } })
hl.on("hyprland.start", function() hl.exec_cmd("hyprpaper") end)

-- Audio stack (also required for Bluetooth audio)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire") end)
hl.on("hyprland.start", function() hl.exec_cmd("wireplumber") end)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire-pulse") end)

-- Bar + dock. eww-state is the only thing that starts eww, opens or closes its
-- windows, or writes its variables; nothing here opens a window itself. That is
-- the whole point of it: two things racing to open the same window is what left
-- the old setup with an inconsistent registry and a bar that would not hide.
hl.on("hyprland.start", function()
    hl.exec_cmd("eww-state daemon")
end)

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

-- Layer surfaces are not animated by default (the leaf ships at speed 0). This
-- is what gives the bar and dock their slide; which way each one goes is the
-- per-namespace rules above. Slower than the windows leaf on purpose: macOS
-- takes about 0.4s to hide the menu bar and Dock, and at speed 3 the slab is
-- gone before the eye follows it.
hl.animation({ leaf = "layers",    enabled = true, speed = 4, bezier = "smooth", style = "slide" })
-- The in/out leaves are set explicitly rather than left to inherit: they each
-- carry their own speed, and `hyprctl animations` reports them still at 0.
hl.animation({ leaf = "layersIn",  enabled = true, speed = 4, bezier = "smooth", style = "slide" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 4, bezier = "smooth", style = "slide" })

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

-- macOS-like window management

-- Float every window by default (mac-style stacking, not tiling)
hl.window_rule({ name = "float-all", match = { class = ".*" }, float = true })

-- Floating windows with no saved geometry map at a tiny default size.
hl.window_rule({ name = "firefox-size", match = { class = "[Ff]irefox" }, size = "1500 950", center = true })

-- A 1px hairline around every window, the same trick the eww menus use: a
-- translucent white edge rather than a drawn colour, dimmer when unfocused.
-- Edges stay grabbable well beyond it via extend_border_grab_area.
-- A window filling the screen has no edge to draw: the hairline would trace the
-- screen border and the rounding would cut the wallpaper in at the corners.
-- Matched on state rather than on a class, so it follows the window in and out.
hl.window_rule({ name = "fullscreen-no-chrome", match = { fullscreen = true }, border_size = 0, rounding = 0 })

hl.config({ general = {
  resize_on_border = true,
  extend_border_grab_area = 15,
  border_size = 1,
  ["col.active_border"] = "rgba(ffffff33)",
  ["col.inactive_border"] = "rgba(ffffff1a)",
} })

-- Blur applies to anything with alpha < 1 (e.g. alacritty). xray = false blurs
-- the wallpaper behind a window rather than other windows.
hl.config({ decoration = {
  rounding = 6,
  blur = { enabled = true, size = 6, passes = 3, new_optimizations = true, xray = false },
} })

-- Mouse: Super + left-drag moves a window, Super + right-drag resizes it.
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- macOS-like fullscreen "Spaces".
-- A window entering a fullscreen state is whisked onto its own empty workspace
-- ("Space") and comes back when it leaves, and the bars hide while one is
-- focused. None of that is here any more: eww-state watches the same compositor
-- event socket and owns where a fullscreen window lives, so this file keeps only
-- what the compositor itself has to know.
hl.config({ general = { gaps_in = 0 } })

-- A 3-finger swipe slides between Spaces. scale is a delta multiplier: lower =
-- more finger travel per switch.
--
-- Guarded because `hyprctl reload` re-runs this file into the same process
-- without clearing what it registered last time, and a doubled gesture jumps
-- two Spaces per swipe.
if not _G.__mac_spaces_init then
    _G.__mac_spaces_init = true
    hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace", scale = 0.5 })
end


-- The Lua config has a permission model; without this, plugins are denied.
-- hyprbars is a local fork (~/files/github/hyprland-plugins, branch macos-buttons)
-- that can rasterize an SVG file into a button instead of drawing a text glyph;
-- it is loaded straight from its build dir, so hyprpm is not in the picture.
-- scripts/hyprbars-build.sh owns it: --auto rebuilds at login whenever the .so
-- was built against another Hyprland, --update pulls upstream's pinned commit and
-- re-applies patches/hyprbars-svg-icons.patch on top.
hl.permission("/usr/(bin|local/bin)/hyprpm", "plugin", "allow")
hl.permission("/home/al/files/github/hyprland-plugins/.*", "plugin", "allow")
hl.permission("/home/al/files/github/blur/.*", "plugin", "allow")


-- Title bars (hyprbars), mac-style. add_button() only appends, and the list is
-- cleared only on a config reload, so declaring the buttons here is idempotent —
-- adding them out-of-band via `hyprctl eval` piles up a new pair every reload.
-- The guard is needed because plugins aren't loaded on the first parse at login;
-- the else branch bootstraps them.
if hl.plugin and hl.plugin.hyprbars ~= nil then
    hl.config({ plugin = { hyprbars = {
        bar_height = 28,
        -- Put the bar inside the window border so the 1px hairline wraps the
        -- title bar too, instead of stopping at the client area.
        bar_precedence_over_border = true,
        -- Alpha d9 matches alacritty's opacity so the bar and body frost alike. A
        -- seamless join isn't possible: hyprbars always draws its own strip above
        -- the client area, so a faint seam remains.
        bar_color = "rgba(2e2e2ed9)",
        bar_blur = true,
        bar_text_size = 0,
        bar_text_font = "Inter",
        bar_text_align = "center",
        bar_buttons_alignment = "left",
        bar_padding = 12,
        bar_button_padding = 8,
        icon_on_hover = true,
        on_double_click = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]],
    } } })

    -- Traffic lights: red = close, yellow = minimize, green = maximize. Actions
    -- run through hyprbars' exec dispatcher, so a Lua dispatcher must be spelled
    -- `hyprctl dispatch "<dispatcher>"` — a bare one is parsed as Lua. Green only
    -- toggles maximize; eww-state does the Space move, off the same event.
    -- Red goes through eww-state because a window closed inside a Space has to
    -- leave it first, or the emptied Space lingers instead of auto-destroying.
    -- Yellow does too, and for a related reason: there is no minimize in
    -- Hyprland, so it is a move onto a workspace nothing draws, and only
    -- eww-state knows where the window came from or has a dock to put it in.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(4d0000)", size = 13, icon = "/home/al/.config/hypr/icons/close.svg", action = "eww-state window close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(febc2e)", fg_color = "rgb(995700)", size = 13, icon = "/home/al/.config/hypr/icons/minimize.svg", action = "eww-state window minimize" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(006500)", size = 13, icon = "/home/al/.config/hypr/icons/maximize.svg", action = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]] })

    -- Title bars are opt-in: a catch-all rule hides them, then these apps get one
    -- back (most apps draw their own chrome). Works because `no_bar` is a
    -- windowEffects rule, where the last matching rule wins. `color` is optional
    -- and blends the bar into that app's own background; nil uses bar_color.
    local titlebar_apps = {
        { class = "Alacritty", color = "rgba(181818d9)" },
        { class = "nemo",      color = "rgba(e8e8ecff)" },
    }

    -- Hide the bar on everything by default...
    hl.window_rule({
        name = "hyprbars-nobar-default",
        match = { class = ".*" },
        ["hyprbars:no_bar"] = true,
    })

    -- ...then opt the allowlisted apps back in (and color their bars).
    for _, app in ipairs(titlebar_apps) do
        hl.window_rule({
            name = "hyprbars-bar-" .. app.class,
            match = { class = "^" .. app.class .. "$" },
            ["hyprbars:no_bar"] = false,
        })
        if app.color then
            hl.window_rule({
                name = "hyprbars-color-" .. app.class,
                match = { class = "^" .. app.class .. "$" },
                ["hyprbars:bar_color"] = app.color,
            })
        end
    end
else
    -- First login: plugins aren't loaded yet, so load hyprbars, poll until it
    -- registers, then reload so the block above runs with the plugin present.
    hl.on("hyprland.start", function()
        hl.exec_cmd("/home/al/.config/hypr/scripts/hyprbars-build.sh --auto --load")
        local tries = 0
        local poll
        poll = hl.timer(function()
            tries = tries + 1
            if hl.plugin and hl.plugin.hyprbars ~= nil then
                poll:set_enabled(false)
                hl.exec_cmd("hyprctl reload")
            elseif tries >= 50 then
                poll:set_enabled(false)
            end
        end, { timeout = 200, type = "repeat" })
    end)
end
