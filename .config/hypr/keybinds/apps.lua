local vars = require "config.vars"
local mod = vars.mod

return {
    { key = mod .. " + Return", action = hl.dsp.exec_cmd(vars.terminal) },
    { key = mod .. " + W",      action = hl.dsp.exec_cmd(vars.browser) },
}
