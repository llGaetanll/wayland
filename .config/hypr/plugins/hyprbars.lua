-- Title bars (hyprbars), mac-style.
--
-- hyprbars is a local fork (~/files/github/hyprland-plugins, branch
-- macos-buttons) that can rasterize an SVG file into a button instead of drawing
-- a text glyph; it is loaded straight from its build dir, so hyprpm is not in
-- the picture. scripts/hyprbars-build.sh owns it: --auto rebuilds at login
-- whenever the .so was built against another Hyprland, --update pulls upstream's
-- pinned commit and re-applies patches/hyprbars-svg-icons.patch on top.

local icons = "/home/al/.config/hypr/icons"

-- Apps that get a title bar (most draw their own chrome). `color` is optional
-- and blends the bar into that app's own background; nil uses bar_color.
local titlebar_apps = {
    { class = "Alacritty", color = "rgba(181818d9)" },
    { class = "nemo",      color = "rgba(e8e8ecff)" },
}

local maximize = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]]

-- add_button() only appends, and the list is cleared only on a config reload, so
-- declaring the buttons here is idempotent -- adding them out-of-band via
-- `hyprctl eval` piles up a new pair every reload. The guard is needed because
-- plugins aren't loaded on the first parse at login; the else branch bootstraps
-- them.
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
        on_double_click = maximize,
    } } })

    -- Traffic lights: red = close, yellow = minimize, green = maximize. Actions
    -- run through hyprbars' exec dispatcher, so a Lua dispatcher must be spelled
    -- `hyprctl dispatch "<dispatcher>"` -- a bare one is parsed as Lua. Green only
    -- toggles maximize; eww-state does the Space move, off the same event.
    -- Red goes through eww-state because a window closed inside a Space has to
    -- leave it first, or the emptied Space lingers instead of auto-destroying.
    -- Yellow does too, and for a related reason: there is no minimize in
    -- Hyprland, so it is a move onto a workspace nothing draws, and only
    -- eww-state knows where the window came from or has a dock to put it in.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(4d0000)", size = 13, icon = icons .. "/close.svg",    action = "eww-state window close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(febc2e)", fg_color = "rgb(995700)", size = 13, icon = icons .. "/minimize.svg", action = "eww-state window minimize" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(006500)", size = 13, icon = icons .. "/maximize.svg", action = maximize })

    -- Title bars are opt-in: a catch-all rule hides them, then the apps above get
    -- one back. Works because `no_bar` is a windowEffects rule, where the last
    -- matching rule wins.
    hl.window_rule({
        name = "hyprbars-nobar-default",
        match = { class = ".*" },
        ["hyprbars:no_bar"] = true,
    })

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
