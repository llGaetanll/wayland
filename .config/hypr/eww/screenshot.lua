local mod = require("config.vars").mod

-- slurp region-select -> grim -> eww preview menu with Copy/Save, all run by
-- eww-state; a second press while the selector is up is dropped by its state
-- machine rather than by a lock file.
hl.bind("Print",               hl.dsp.exec_cmd("eww-state shot capture"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("eww-state shot capture"))

-- slurp's dim overlay is a layer surface, so the layers leaf in
-- config/animations.lua slides it down the screen before the region can be
-- picked. The selector has to be there the instant the key is pressed.
hl.layer_rule({ name = "slurp-no-anim", match = { namespace = "^selection$" }, no_anim = true })
