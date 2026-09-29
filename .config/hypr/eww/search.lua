local mod = require("config.vars").mod

-- Spotlight-style launcher, served by eww-state's `search` module. Replaced
-- rofi on 2026-09-28; the old theme is still in ~/.config/rofi/ for reference.
hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))

-- The launcher's keyboard: eww has no key bindings, so only the compositor can
-- see a key the text field does not want. While this submap is active these four
-- keys come here and everything else still reaches the entry.
-- Escape also resets the submap, so a dead daemon cannot leave the session stuck.
-- KEEP IN SYNC with SUBMAP in eww-state (crates/module-search/src/perform.rs).
hl.define_submap("search", function()
    hl.bind("escape",          hl.dsp.exec_cmd("eww-state search close"))
    hl.bind("escape",          hl.dsp.submap("reset"))
    hl.bind("up",              hl.dsp.exec_cmd("eww-state search move -1"), { repeating = true })
    hl.bind("down",            hl.dsp.exec_cmd("eww-state search move 1"),  { repeating = true })
    hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))
end)

-- no_anim: the launcher resizes its surface on every keystroke, and animating
-- that makes the text visibly morph. Costs the open fade too.
hl.layer_rule({ name = "eww-search-blur", match = { namespace = "^eww-search$" }, blur = true, ignore_alpha = 0.2, no_anim = true })
