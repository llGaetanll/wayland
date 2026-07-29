
local mod = "SUPER"

local terminal = os.getenv("TERMINAL") or "alacritty"
local browser = os. getenv("BROWSER") or "firefox"

-- Close via mac-fullscreen.sh so a fullscreen-Space window also returns to the
-- desktop Space and lets the emptied Space auto-destroy.
hl.bind(mod .. " + Q",      hl.dsp.exec_cmd("/home/al/.config/hypr/scripts/mac-fullscreen.sh close"))
hl.bind(mod .. " + Return", hl.dsp.exec_cmd(terminal))
hl.bind(mod .. " + W", hl.dsp.exec_cmd(browser))

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
-- no_warps: Hyprland otherwise yanks the cursor to the centre of a newly focused
-- window, which is jarring when an app is sent to its own Space.
hl.config({ cursor = { no_hardware_cursors = true, no_warps = true } })
hl.on("hyprland.start", function() hl.exec_cmd("hyprpaper") end)

-- Audio stack (also required for Bluetooth audio)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire") end)
hl.on("hyprland.start", function() hl.exec_cmd("wireplumber") end)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire-pulse") end)

-- Bar + dock (~/.config/eww). Start the daemon explicitly and open both windows
-- against it in ONE handler: two handlers relying on `eww open` to auto-start the
-- daemon race, and the survivor's window registry ends up inconsistent — so
-- `eww close bar` from sync-bars.sh silently no-ops and the bar never hides.
hl.on("hyprland.start", function()
    hl.exec_cmd("sh -c 'eww daemon; eww open bar; eww open dock'")
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
-- Any window entering a fullscreen state is whisked onto its own empty workspace
-- ("Space") and comes back when it leaves. Driven by the window.fullscreen event
-- rather than a button, so every route in behaves the same — green traffic-light,
-- double-click, or an app's own F11 — including apps with no hyprbars button.
-- The bars hide while a Space is focused; a 3-finger swipe slides between Spaces.
hl.config({ general = { gaps_in = 0 } })

-- Guarded so `hyprctl reload` (which re-runs this file) can't stack duplicate
-- handlers — a doubled swipe gesture would jump two Spaces at once.
if not _G.__mac_spaces_init then
    _G.__mac_spaces_init = true

    local sync_bars = "/home/al/.config/hypr/scripts/sync-bars.sh"

    -- The workspace a window came from, one file per window address. On disk
    -- rather than in memory so it survives `hyprctl reload` and is readable by
    -- sync-bars.sh and mac-fullscreen.sh.
    local runtime = (os.getenv("XDG_RUNTIME_DIR") or "/tmp") .. "/mac-fs"
    hl.exec_cmd("mkdir -p '" .. runtime .. "'")
    local function origin_path(addr) return runtime .. "/" .. addr end
    local function origin_get(addr)
        local f = io.open(origin_path(addr), "r"); if not f then return nil end
        local v = f:read("*a"); f:close()
        return (v and v ~= "") and v or nil
    end
    local function origin_set(addr, ws)
        local f = io.open(origin_path(addr), "w"); if not f then return end
        f:write(tostring(ws)); f:close()
    end
    local function origin_clear(addr) os.remove(origin_path(addr)) end

    -- Move as an INSTANT cut: kill the workspaces slide for this one move (the
    -- frames are identical, so the cut is invisible), then restore it so 3-finger
    -- swipes still animate. The re-enable must be deferred — flipping it back in
    -- the same tick renders the switch with the slide already restored.
    local function move_instant(addr, ws)
        hl.animation({ leaf = "workspaces", enabled = false })
        hl.dispatch(hl.dsp.window.move({ workspace = ws, follow = true, window = "address:" .. addr }))
        hl.timer(function()
            hl.animation({ leaf = "workspaces", enabled = true, speed = 8, bezier = "default" })
        end, { timeout = 100, type = "oneshot" })
    end

    -- Moving a fullscreen window makes Hyprland re-emit window.fullscreen (a
    -- spurious fs=0 then fs=1 on the destination workspace); acting on those
    -- echoes bounces the window forever. So events never act directly: each bumps
    -- a per-window generation token and arms a timer, and only the last event in a
    -- burst survives to reconcile against the settled state.
    local gen = {}
    local function reconcile(addr)
        local cur = hl.get_window("address:" .. addr)
        if not cur then origin_clear(addr); gen[addr] = nil; return end   -- window vanished
        local fs = cur.fullscreen or 0
        local origin = origin_get(addr)

        if fs ~= 0 and not origin then
            -- ENTER: record the origin workspace, then let the grow animation play
            -- before instant-cutting onto an empty Space. The bars were already
            -- hidden by the raw handler, which freed their reserved area so the
            -- window could grow edge-to-edge first — the precondition for a
            -- frame-identical cut. The post-move sync rides on workspace.active.
            origin_set(addr, (cur.workspace and cur.workspace.id) or 1)
            hl.timer(function()
                local c = hl.get_window("address:" .. addr)
                if c and (c.fullscreen or 0) ~= 0 and origin_get(addr) then
                    move_instant(addr, "empty")
                end
            end, { timeout = 250, type = "oneshot" })
        elseif fs == 0 and origin then
            -- EXIT: cut back to origin, which empties the Space so Hyprland
            -- auto-destroys it.
            origin_clear(addr)
            move_instant(addr, origin)
        end
        -- fs~=0 & origin   -> already in a Space (spurious re-fullscreen): no-op
        -- fs==0 & !origin  -> ordinary window toggling maximize off: no-op
    end

    hl.on("window.fullscreen", function(w)
        if type(w) ~= "table" or not w.address then w = hl.get_active_window() end
        if not w or not w.address then return end
        local addr = w.address
        -- Hide the bars immediately on the way IN, without waiting for the
        -- debounced move. Only on entry: on exit the bars come back later via
        -- workspace.active, so they don't flash in mid-shrink.
        if (w.fullscreen or 0) ~= 0 then hl.exec_cmd(sync_bars) end
        gen[addr] = (gen[addr] or 0) + 1
        local mine = gen[addr]
        hl.timer(function()
            if gen[addr] == mine then reconcile(addr) end
        end, { timeout = 140, type = "oneshot" })
    end)

    -- Every Space enter/exit moves with follow=true, so this covers both, plus
    -- 3-finger swipes. window.close is the case with no workspace switch.
    hl.on("workspace.active", function() hl.exec_cmd(sync_bars) end)
    hl.on("window.close",     function() hl.exec_cmd(sync_bars) end)

    -- scale is a delta multiplier: lower = more finger travel per switch.
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

    -- Traffic lights: red = close, green = maximize. Actions run through hyprbars'
    -- exec dispatcher, so a Lua dispatcher must be spelled
    -- `hyprctl dispatch "<dispatcher>"` — a bare one is parsed as Lua. Green only
    -- toggles maximize; the window.fullscreen handler above does the Space move.
    -- Red goes through mac-fullscreen.sh because closing inside a Space must hop
    -- back to the desktop Space first so the emptied Space auto-destroys.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(2e2e2e)", size = 11, icon = "×", action = "/home/al/.config/hypr/scripts/mac-fullscreen.sh close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(2e2e2e)", size = 11, icon = "+", action = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]] })

    -- Title bars are opt-in: a catch-all rule hides them, then these apps get one
    -- back (most apps draw their own chrome). Works because `no_bar` is a
    -- windowEffects rule, where the last matching rule wins. `color` is optional
    -- and blends the bar into that app's own background; nil uses bar_color.
    local titlebar_apps = {
        { class = "Alacritty", color = "rgba(181818d9)" },
        { class = "nemo",      color = nil            },
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
