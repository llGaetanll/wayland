
local mod = "SUPER"

local terminal = os.getenv("TERMINAL") or "alacritty"
local browser = os. getenv("BROWSER") or "firefox"

-- Basic Keybinds 
hl.bind(mod .. " + Q",      hl.dsp.window.close())
hl.bind(mod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mod .. " + W", hl.dsp.exec_cmd(browser))

-- App launcher: mac-style Spotlight (SUPER + Space). Config + theme live in
-- ~/.config/rofi/{config.rasi,themes/spotlight.rasi}. rofi runs as a Wayland
-- layer surface (namespace "rofi"), so the float-all window rule doesn't touch
-- it; the layer_rule below frosts the translucent background instead.
hl.bind(mod .. " + Space", hl.dsp.exec_cmd("rofi -show drun"))
-- no_anim: rofi resizes its layer surface on every keystroke (dynamic list).
-- Letting Hyprland animate that resize produces a "morphing/bending text"
-- effect, so animation is disabled for this layer — resizes snap instantly.
-- Trade-off: no fade-in on open either (the launcher just appears).
hl.layer_rule({ name = "rofi-blur", match = { namespace = "^rofi$" }, blur = true, ignore_alpha = 0.5, no_anim = true })

-- Blur eww surfaces (bar, dock, menus) — they share the gtk-layer-shell layer
-- namespace. The full-screen menu backdrop is deliberately given its OWN
-- namespace ("eww-backdrop", see eww.yuck) so it is EXCLUDED here: blurring a
-- 1920x1200 surface every frame near-locks the machine.
hl.layer_rule({ name = "eww-blur", match = { namespace = "^gtk-layer-shell$" }, blur = true, ignore_alpha = 0.2 })

-- Volume: SUPER +/-  (locked = works on lockscreen, repeating = holds to ramp)
-- The eww bar polls its volume icon every 2s (see eww.yuck), so the change is
-- reflected there on its own — no need to signal the bar from the keybind.
hl.bind(mod .. " + equal", hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + plus",  hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + minus", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),       { locked = true, repeating = true })

-- Brightness: SUPER + ALT +/-  (needs brightnessctl; eww polls its icon too)
hl.bind(mod .. " + ALT + equal", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + plus",  hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + minus", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"), { locked = true, repeating = true })

-- Screenshot: Print (the PrtSc key) → slurp crosshair region-select → grim capture
-- → an eww preview menu (top-right) with Copy / Save actions. SUPER+SHIFT+S is a
-- convenient alias (macOS uses SHIFT+CMD-family combos). See screenshot.sh.
hl.bind("Print",               hl.dsp.exec_cmd("/home/al/.config/eww/scripts/screenshot.sh"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("/home/al/.config/eww/scripts/screenshot.sh"))

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
hl.config({ cursor = { no_hardware_cursors = true } })
hl.on("hyprland.start", function() hl.exec_cmd("hyprpaper") end)

-- Audio stack (PipeWire + WirePlumber) — also required for Bluetooth audio
hl.on("hyprland.start", function() hl.exec_cmd("pipewire") end)
hl.on("hyprland.start", function() hl.exec_cmd("wireplumber") end)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire-pulse") end)

-- Top Bar: eww window (~/.config/eww), matching the eww dock. `eww open`
-- auto-starts the daemon if it isn't already running.
hl.on("hyprland.start", function() hl.exec_cmd("eww open bar") end)

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

-- ============================================================
-- macOS-like window management
-- ============================================================

-- Float every window by default (mac-style stacking, not tiling)
hl.window_rule({ name = "float-all", match = { class = ".*" }, float = true })

-- Floating windows with no saved geometry (e.g. a fresh Firefox window) otherwise
-- map at a tiny default size. Give Firefox a sensible size and center it on open.
hl.window_rule({ name = "firefox-size", match = { class = "[Ff]irefox" }, size = "1500 950", center = true })

-- Resize floating windows by dragging their edges/corners (mac-like).
-- extend_border_grab_area makes the grabbable edge wider than the visible border.
-- border_size = 0 removes the (white) active-window border; grabbing still
-- works via extend_border_grab_area.
hl.config({ general = { resize_on_border = true, extend_border_grab_area = 15, border_size = 0 } })

-- Rounded corners (all four — Hyprland rounding is uniform, no per-corner).
hl.config({ decoration = { rounding = 6 } })

-- Mouse: Super + left-drag moves a window, Super + right-drag resizes it.
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- Allow hyprpm to load plugins. The Lua config has a permission model;
-- without this, plugins (e.g. hyprbars for title bars) are denied.
hl.permission("/usr/(bin|local/bin)/hyprpm", "plugin", "allow")

-- mac-style dock: an eww window pinned to the bottom (~/.config/eww).
-- `eww open` auto-starts the daemon if it isn't already running.
hl.on("hyprland.start", function()
    hl.exec_cmd("eww open dock")
end)

-- Title bars (hyprbars), mac-style.
--
-- hyprbars' button list is additive: add_button() only ever appends, and the
-- list is cleared in exactly one place internally — on a Hyprland config reload
-- (onPreConfigReload). So declaring the buttons here, in the config, makes them
-- idempotent: every reload clears the list and this block re-adds exactly these
-- two. (Adding them out-of-band via `hyprctl eval` is what made a fresh red/green
-- pair pile up on every reload.)
--
-- The guard is required because hyprpm plugins are NOT loaded yet when this
-- config is first parsed at login, so hl.plugin.hyprbars is nil on that pass.
-- The bootstrap in the else branch loads the plugin and then triggers a reload,
-- which re-runs this block with the plugin present.
if hl.plugin and hl.plugin.hyprbars ~= nil then
    hl.config({ plugin = { hyprbars = {
        bar_height = 26,
        bar_color = "rgb(2e2e2e)",
        bar_text_size = 0,
        bar_text_font = "Inter",
        bar_text_align = "center",
        bar_buttons_alignment = "left",
        bar_padding = 10,
        bar_button_padding = 8,
        icon_on_hover = true,
        on_double_click = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]],
    } } })

    -- Traffic-light buttons (left side, mac order): red = close, green = maximize.
    -- Glyphs only show on hover (icon_on_hover), so they normally read as colored dots.
    -- Actions run through hyprbars' `exec` dispatcher, so they must be shell commands.
    -- Under Hyprland's Lua config, `hyprctl dispatch <old-dispatcher>` no longer works
    -- (it's parsed as Lua now), so the action must be `hyprctl dispatch "<lua dispatcher>"`.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(2e2e2e)", size = 11, icon = "×", action = [[hyprctl dispatch "hl.dsp.window.close()"]] })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(2e2e2e)", size = 11, icon = "+", action = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]] })

    -- Per-app title bar colors: make each app's bar blend into that app's own
    -- background instead of the global bar_color above. hyprbars exposes a
    -- per-window `hyprbars:bar_color` effect; a window rule keyed on class sets it.
    -- To theme another app, add a `[class] = color` entry — nothing else to change.
    --
    -- Alacritty has no [colors.primary] in its config, so it uses the built-in
    -- default background #181818 (alacritty 0.17). Update this if you set a theme.
    local bar_colors = {
        Alacritty = "rgb(181818)",
    }
    for class, color in pairs(bar_colors) do
        hl.window_rule({
            name = "hyprbars-color-" .. class,
            match = { class = "^" .. class .. "$" },
            ["hyprbars:bar_color"] = color,
        })
    end

    -- Firefox: no hyprbars bar. Firefox draws its own tab strip as the title bar,
    -- and userChrome.css (~/.config/firefox/chrome/userChrome.css) puts macOS-style
    -- traffic-light buttons inline to the left of the tabs. hyprbars would just be
    -- a redundant strip above that, so hide it for Firefox windows.
    hl.window_rule({
        name = "hyprbars-nobar-firefox",
        match = { class = "[Ff]irefox" },
        ["hyprbars:no_bar"] = true,
    })
else
    -- First login: load the (enabled but not-yet-loaded) hyprbars plugin, poll
    -- until it registers, then trigger one `hyprctl reload` so the block above
    -- runs with the plugin present. The reload wipes this timer, so it can't loop;
    -- we also stop it ourselves and give up after ~10s if the plugin never shows.
    hl.on("hyprland.start", function()
        hl.exec_cmd("hyprpm reload -n")
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
