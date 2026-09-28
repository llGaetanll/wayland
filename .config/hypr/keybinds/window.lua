local mod = require("config.vars").mod

return {
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
