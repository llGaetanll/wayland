-- Mac-style title bars from a local hyprbars fork
-- (~/files/github/hyprland-plugins, branch macos-buttons) that rasterizes SVG
-- buttons instead of text glyphs, loaded straight from its build dir.
-- scripts/hyprbars-build.sh owns it: --auto rebuilds at login when the .so was
-- built against another Hyprland, --update re-applies the patch on upstream.

local icons = "/home/al/.config/hypr/icons"

-- Apps that get a title bar (most draw their own chrome). `color` blends the bar
-- into that app's background; nil uses bar_color.
local titlebar_apps = {
    { class = "Alacritty", color = "rgba(181818d9)" },
    { class = "nemo",      color = "rgba(e8e8ecff)" },
}

local maximize = [[hyprctl dispatch "hl.dsp.window.fullscreen({ mode = 'maximized' })"]]

-- Declaring the buttons here is idempotent: add_button() only appends, and the
-- list is cleared on config reload. The guard is for the first parse at login,
-- before plugins are loaded; the else branch bootstraps them.
if hl.plugin and hl.plugin.hyprbars ~= nil then
    hl.config({ plugin = { hyprbars = {
        bar_height = 28,
        -- Wraps the 1px hairline around the title bar too.
        bar_precedence_over_border = true,
        -- Alpha d9 matches alacritty's opacity so bar and body frost alike. A
        -- faint seam remains; hyprbars always draws its own strip.
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
    -- `hyprctl dispatch "<dispatcher>"` -- a bare one is parsed as Lua.
    -- Red and yellow go through eww-state: it owns Spaces (a window must leave
    -- its Space before closing) and minimize, which Hyprland has no notion of.
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(ff5f57)", fg_color = "rgb(4d0000)", size = 13, icon = icons .. "/close.svg",    action = "eww-state window close" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(febc2e)", fg_color = "rgb(995700)", size = 13, icon = icons .. "/minimize.svg", action = "eww-state window minimize" })
    hl.plugin.hyprbars.add_button({ bg_color = "rgb(28c840)", fg_color = "rgb(006500)", size = 13, icon = icons .. "/maximize.svg", action = maximize })

    -- Opt-in: a catch-all hides every bar, then the apps above get one back.
    -- Works because `no_bar` is a windowEffects rule, where the last match wins.
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
    -- First login: load hyprbars, poll until it registers, then reload so the
    -- block above runs with the plugin present.
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
