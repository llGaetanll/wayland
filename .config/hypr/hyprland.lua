
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

-- Spotlight-style launcher; theme in ~/.config/rofi/.
hl.bind(mod .. " + Space", hl.dsp.exec_cmd("rofi -show drun"))
-- no_anim: rofi resizes its layer surface on every keystroke, and animating that
-- makes the text visibly morph. Costs the open fade too.
hl.layer_rule({ name = "rofi-blur", match = { namespace = "^rofi$" }, blur = true, ignore_alpha = 0.5, no_anim = true })

-- Blur eww surfaces (bar, dock, menus). The full-screen menu backdrop uses its
-- own "eww-backdrop" namespace to stay out of this — blurring a fullscreen
-- surface every frame near-locks the machine.
hl.layer_rule({ name = "eww-blur", match = { namespace = "^gtk-layer-shell$" }, blur = true, ignore_alpha = 0.2 })

-- Volume: SUPER +/-  (locked = works on lockscreen, repeating = holds to ramp)
hl.bind(mod .. " + equal", hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + plus",  hl.dsp.exec_cmd("wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + minus", hl.dsp.exec_cmd("wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"),       { locked = true, repeating = true })

-- Brightness: SUPER + ALT +/-  (needs brightnessctl)
hl.bind(mod .. " + ALT + equal", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + plus",  hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%+"), { locked = true, repeating = true })
hl.bind(mod .. " + ALT + minus", hl.dsp.exec_cmd("brightnessctl -e4 -n2 set 5%-"), { locked = true, repeating = true })

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

-- border_size = 0 hides the active-window border; edges stay grabbable via
-- extend_border_grab_area.
hl.config({ general = { resize_on_border = true, extend_border_grab_area = 15, border_size = 0 } })

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
hl.permission("/usr/(bin|local/bin)/hyprpm", "plugin", "allow")


-- Title bars (hyprbars), mac-style. add_button() only appends, and the list is
-- cleared only on a config reload, so declaring the buttons here is idempotent —
-- adding them out-of-band via `hyprctl eval` piles up a new pair every reload.
-- The guard is needed because plugins aren't loaded on the first parse at login;
-- the else branch bootstraps them.
if hl.plugin and hl.plugin.hyprbars ~= nil then
    hl.config({ plugin = { hyprbars = {
        bar_height = 26,
        -- Alpha d9 matches alacritty's opacity so the bar and body frost alike. A
        -- seamless join isn't possible: hyprbars always draws its own strip above
        -- the client area, so a faint seam remains.
        bar_color = "rgba(2e2e2ed9)",
        bar_blur = true,
        bar_text_size = 0,
        bar_text_font = "Inter",
        bar_text_align = "center",
        bar_buttons_alignment = "left",
        bar_padding = 10,
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
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(2e2e2e)", size = 11, icon = "×", action = "eww-state window close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(febc2e)", fg_color = "rgb(2e2e2e)", size = 11, icon = "−", action = "eww-state window minimize" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(2e2e2e)", size = 11, icon = "+", action = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]] })

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
