local mod = require("config.vars").mod

-- slurp region-select -> grim -> eww preview menu with Copy/Save. eww-state runs
-- the capture and stages the result; a second press while the selector is up is
-- dropped by its state machine rather than by a lock file.
hl.bind("Print",               hl.dsp.exec_cmd("eww-state shot capture"))
hl.bind(mod .. " + SHIFT + S", hl.dsp.exec_cmd("eww-state shot capture"))
