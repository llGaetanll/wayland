-- Push-to-talk dictation on the Copilot key. The key is not its own keysym: the
-- firmware sends SUPER+SHIFT+F23, and holding it autorepeats that chord, so
-- press and release are distinguishable and hold-to-talk works.
local mod = require("config.vars").mod

local dictate = "/home/al/.config/hypr/scripts/dictate.sh"
local copilot = mod .. " + SHIFT + F23"

return {
    { key = copilot, action = hl.dsp.exec_cmd(dictate .. " start") },
    { key = copilot, action = hl.dsp.exec_cmd(dictate .. " stop"), opts = { release = true } },
}
