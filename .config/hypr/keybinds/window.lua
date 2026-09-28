local mod = require("config.vars").mod
local rules = require("config.rules")

-- Floating vs tiling is a global mode, not a per-window state: the float-all
-- rule decides what new windows do, and every window that already exists is
-- switched over to match, on every workspace.
local function set_floating(floating)
    rules.float_all:set_enabled(floating)
    local action = floating and "on" or "off"
    for _, w in ipairs(hl.get_windows()) do
        hl.dispatch(hl.dsp.window.float({ window = "address:" .. w.address, action = action }))
    end
end

return {
    -- Everything floats / everything tiles.
    { key = mod .. " + ALT + S", action = function() set_floating(true) end },
    { key = mod .. " + ALT + T", action = function() set_floating(false) end },

    -- Close through eww-state: a window in a fullscreen Space has to be left
    -- behind before it is closed, or the emptied Space lingers instead of
    -- auto-destroying.
    { key = mod .. " + Q", action = hl.dsp.exec_cmd("eww-state window close") },

    -- Toggle fullscreen on the focused window (moves it to its own Space).
    { key = mod .. " + ALT + F", action = hl.dsp.window.fullscreen({ mode = "fullscreen" }) },

    -- Super + left-drag moves a window, Super + right-drag resizes it.
    { key = mod .. " + mouse:272", action = hl.dsp.window.drag(),   opts = { mouse = true } },
    { key = mod .. " + mouse:273", action = hl.dsp.window.resize(), opts = { mouse = true } },
}
