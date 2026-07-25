
local mod = "SUPER"

local terminal = os.getenv("TERMINAL") or "alacritty"
local browser = os. getenv("BROWSER") or "firefox"

-- Basic Keybinds 
-- Close via the same close-aware path as the red traffic-light button, so
-- SUPER+Q on a fullscreen-Space window also returns to the desktop Space and
-- lets the emptied Space auto-destroy (see mac-fullscreen.sh).
hl.bind(mod .. " + Q",      hl.dsp.exec_cmd("/home/al/.config/hypr/scripts/mac-fullscreen.sh close"))
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
-- no_warps: never teleport the cursor. Hyprland otherwise warps it to the centre
-- of a newly focused window — which, when a fullscreen app is sent to its own
-- Space, yanked the cursor to mid-screen and broke the illusion. macOS never
-- warps the cursor, and this setup is mouse-driven (no focus-by-direction binds),
-- so disabling warps globally is both the fix and the right default here.
hl.config({ cursor = { no_hardware_cursors = true, no_warps = true } })
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
-- Window blur applies to anything rendered with alpha < 1 (e.g. alacritty
-- opacity = 0.85). new_optimizations improves perf; xray = false blurs the
-- desktop/wallpaper behind a window rather than other windows.
hl.config({ decoration = {
  rounding = 6,
  blur = { enabled = true, size = 6, passes = 3, new_optimizations = true, xray = false },
} })

-- Mouse: Super + left-drag moves a window, Super + right-drag resizes it.
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(),   { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

-- ============================================================
-- macOS-like fullscreen "Spaces"
-- ============================================================
-- Any window that enters a Hyprland fullscreen state gets whisked onto its own
-- empty workspace ("Space"), and comes back when it leaves fullscreen. This is
-- driven entirely by the window.fullscreen event handler below — NOT by a button
-- running a script — so EVERY route into fullscreen behaves identically, which is
-- what finally makes it work for apps like Firefox that have no hyprbars button:
--   * green traffic-light / double-click a title bar -> MAXIMIZE (mode 1): fills
--     the screen but KEEPS the title bar, so you can click green again to leave.
--   * an app's own real fullscreen (Firefox F11, a video) -> mode 2: no chrome;
--     leave it the way you entered (F11 / Esc).
-- Either way the handler moves it to a Space. The two bars (top bar + dock) hide
-- while a Space is focused and return on your desktop Space, like macOS. A
-- 3-finger horizontal swipe slides between Spaces. gaps_in = 0 makes a maximized
-- window edge-to-edge; with everything floating that only affects maximized ones.
hl.config({ general = { gaps_in = 0 } })

-- The event/gesture registrations are guarded so a `hyprctl reload` (which
-- re-runs this whole file) can't stack duplicate handlers — a second identical
-- swipe gesture would make one swipe jump two Spaces, and a second fullscreen
-- handler would move the window to a Space twice.
if not _G.__mac_spaces_init then
    _G.__mac_spaces_init = true

    local sync_bars = "/home/al/.config/hypr/scripts/sync-bars.sh"

    -- Per-window origin tracking. We persist the workspace a window came from to
    -- a file named after its address under $XDG_RUNTIME_DIR/mac-fs/ — the SAME
    -- files sync-bars.sh and mac-fullscreen.sh (close) read. On disk rather than
    -- an in-memory table so it survives a `hyprctl reload` and is visible to
    -- those shell scripts. (io.open/os.remove work in Hyprland's lua sandbox.)
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

    -- Move a window to a workspace as an INSTANT cut: disable the workspaces
    -- slide for just this one move (so a full-screen window doesn't visibly fly
    -- in from the side — the frames are identical, so the cut is invisible), then
    -- restore the slide so the 3-finger swipe between Spaces still animates.
    -- workspaces normally inherits `global` (speed 8, bezier default).
    --
    -- The re-enable MUST be deferred, not inline: if we flip the animation back on
    -- in the same tick as the move, Hyprland renders the workspace switch with the
    -- slide already restored — the exact jitter we're removing. (The old script got
    -- away with an inline re-enable only because each step was a separate `hyprctl`
    -- round-trip, which spaced them across ticks; in-process we must space it out
    -- ourselves.) The switch itself commits within a frame, so 100ms is ample, and
    -- the swipe gesture is exceedingly unlikely to land in that window.
    local function move_instant(addr, ws)
        hl.animation({ leaf = "workspaces", enabled = false })
        hl.dispatch(hl.dsp.window.move({ workspace = ws, follow = true, window = "address:" .. addr }))
        hl.timer(function()
            hl.animation({ leaf = "workspaces", enabled = true, speed = 8, bezier = "default" })
        end, { timeout = 100, type = "oneshot" })
    end

    -- THE Spaces engine. React to Hyprland's own fullscreen state so every route
    -- into fullscreen (green button's maximize, double-click, an app's F11 / a
    -- video) is handled the same — that's what makes it work for apps like Firefox
    -- that have no hyprbars button.
    --
    -- The hard part: moving a full-screen window makes Hyprland RE-EMIT
    -- window.fullscreen (a spurious fs=0 then fs=1 as it re-applies fullscreen on
    -- the destination workspace). Acting on those echoes bounces the window
    -- forever. So we never act on an event directly — every event bumps a
    -- per-window generation token and arms a short timer; only the LAST event in a
    -- burst survives (its gen still matches) and reconciles against the SETTLED
    -- state. A flurry of spurious flips collapses into one evaluation that sees the
    -- final fs, and — because the origin file marks a window as "already in a
    -- Space" — that evaluation is a no-op.
    local gen = {}
    local function reconcile(addr)
        local cur = hl.get_window("address:" .. addr)
        if not cur then origin_clear(addr); gen[addr] = nil; return end   -- window vanished
        local fs = cur.fullscreen or 0
        local origin = origin_get(addr)

        if fs ~= 0 and not origin then
            -- ENTER: not in a Space yet. Record where it came from, then once the
            -- grow (~300ms) has played, instant-cut it onto an empty Space. The
            -- bars are already hidden — the raw handler hid them the instant the
            -- fullscreen event arrived, which also freed their reserved area so the
            -- window grew edge-to-edge (the pre-condition for a frame-identical
            -- cut). No sync_bars after the move either: the workspace switch fires
            -- workspace.active, which syncs the bars. The move re-emits fullscreen
            -- echoes; the debounce funnels them back to the no-op branch (fs~=0 &
            -- origin).
            origin_set(addr, (cur.workspace and cur.workspace.id) or 1)
            hl.timer(function()
                local c = hl.get_window("address:" .. addr)
                if c and (c.fullscreen or 0) ~= 0 and origin_get(addr) then
                    move_instant(addr, "empty")
                end
            end, { timeout = 250, type = "oneshot" })
        elseif fs == 0 and origin then
            -- EXIT: settled out of fullscreen. Cut back to origin (which empties
            -- the Space, so Hyprland auto-destroys it) where it finishes shrinking.
            -- The window isn't full-screen here, so this move emits no echoes; the
            -- bar sync again rides on the workspace.active handler below.
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
        -- Hide the bars IMMEDIATELY on the way INTO fullscreen — don't wait for the
        -- debounced move. sync_bars keys on live fullscreen state, so it hides the
        -- instant the focused workspace is full-screen. We only fire this on entry
        -- (fs~=0): on the way out we deliberately let the bars come back later, via
        -- the post-move workspace.active, so they reappear once the window has
        -- landed back on its desktop Space rather than flashing in mid-exit.
        if (w.fullscreen or 0) ~= 0 then hl.exec_cmd(sync_bars) end
        gen[addr] = (gen[addr] or 0) + 1
        local mine = gen[addr]
        hl.timer(function()
            if gen[addr] == mine then reconcile(addr) end
        end, { timeout = 140, type = "oneshot" })
    end)

    -- The bars' single source of truth after any workspace change. Every Space
    -- enter/exit above moves with follow=true, i.e. it switches workspace, so this
    -- one handler is what actually re-syncs the bars post-move for both — and it
    -- also covers 3-finger swipes between Spaces. window.close is the separate case:
    -- a fullscreen app that's closed gives no workspace switch, but must still hand
    -- the bars back.
    hl.on("workspace.active", function() hl.exec_cmd(sync_bars) end)
    hl.on("window.close",     function() hl.exec_cmd(sync_bars) end)

    -- 3-finger horizontal swipe = move between workspaces (Spaces), macOS-style.
    -- scale is a delta multiplier: < 1 makes the workspaces track your fingers more
    -- slowly, so you must swipe FURTHER to switch — i.e. less sensitive / less
    -- twitchy. 0.5 ≈ double the finger travel of the default; lower it toward ~0.3
    -- for even less sensitivity, or raise back toward 1.0 for more.
    hl.gesture({ fingers = 3, direction = "horizontal", action = "workspace", scale = 0.5 })
end

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
        -- Translucent bar + bar_blur so the title bar frosts the same way the
        -- window body does. Alpha (d9 ≈ 0.85) matches alacritty's opacity so the
        -- bar and terminal read as close to one surface as hyprbars allows. (A
        -- fully seamless join isn't possible: hyprbars always gives the bar its own
        -- strip ABOVE the client area — bar_part_of_window only governs shadows,
        -- not whether the window draws under the bar — so the bar and body are
        -- always two independently-blurred rects with a faint seam between them.)
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

    -- Traffic-light buttons (left side, mac order): red = close, green = fullscreen.
    -- Glyphs only show on hover (icon_on_hover), so they normally read as colored
    -- dots. Actions run through hyprbars' `exec` dispatcher, so they must be shell
    -- commands. Under Hyprland's Lua config a dispatcher action must be spelled
    -- `hyprctl dispatch "<lua dispatcher>"` (bare dispatchers are parsed as Lua).
    --
    -- Green just TOGGLES maximize. It no longer knows anything about Spaces — the
    -- window.fullscreen handler (see the "macOS-like fullscreen Spaces" section
    -- above) reacts to the resulting fullscreen state and does the Space move, so
    -- clicking green and pressing F11 go through the exact same path.
    --
    -- Red = close. Still routed through mac-fullscreen.sh (close), because closing
    -- a window in its own Space must first hop back to the desktop Space so the
    -- emptied Space auto-destroys — that's multi-step, more than a button can do.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(2e2e2e)", size = 11, icon = "×", action = "/home/al/.config/hypr/scripts/mac-fullscreen.sh close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(2e2e2e)", size = 11, icon = "+", action = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]] })

    -- Title bars are OPT-IN. hyprbars draws a bar on every window by default, so
    -- we invert that: a catch-all rule hides the bar everywhere, then each app in
    -- the allowlist below re-enables it. Most apps (Firefox, browsers, etc.) draw
    -- their own chrome, so a hyprbars strip on top would just be redundant; only
    -- the apps that have no titlebar of their own are opted back in here.
    --
    -- This relies on hyprbars' `no_bar` being a windowEffects rule (last matching
    -- rule wins): the per-app `no_bar = false` below overrides the catch-all
    -- `no_bar = true` for that class. To give an app a title bar, add an entry to
    -- `titlebar_apps` — nothing else to change.
    --
    -- `color` (optional) sets a per-app bar color so the bar blends into that app's
    -- own background instead of the global bar_color above (nil = use bar_color).
    -- Alacritty has no [colors.primary] in its config, so it uses the built-in
    -- default background #181818 (alacritty 0.17); the d9 alpha (≈ 0.85) matches
    -- alacritty's own window opacity so the bar and terminal body frost alike.
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
