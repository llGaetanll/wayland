local mod = require("config.vars").mod

-- Spotlight-style launcher, served by eww-state's `search` module. Replaced
-- rofi on 2026-09-28; the old theme is still in ~/.config/rofi/ for reference.
hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))

-- The launcher's keyboard. eww has no key bindings of any kind, so the only
-- thing that can see a key the text field does not want is the compositor.
-- While this submap is active the four keys below come here and every other
-- key still reaches the entry, which is what lets the user keep typing.
--
-- Escape gives the keyboard back here as well as telling the daemon, so a
-- daemon that has died cannot leave the session stuck in a submap.
-- KEEP IN SYNC with SUBMAP in eww-state (crates/module-search/src/perform.rs),
-- which is what enters and leaves it.
hl.define_submap("search", function()
    hl.bind("escape",          hl.dsp.exec_cmd("eww-state search close"))
    hl.bind("escape",          hl.dsp.submap("reset"))
    hl.bind("up",              hl.dsp.exec_cmd("eww-state search move -1"), { repeating = true })
    hl.bind("down",            hl.dsp.exec_cmd("eww-state search move 1"),  { repeating = true })
    hl.bind(mod .. " + Space", hl.dsp.exec_cmd("eww-state search open"))
end)

-- no_anim: the launcher resizes its layer surface on every keystroke as the
-- result list grows and shrinks, and animating that makes the text visibly
-- morph. Same reason rofi had this rule. Costs the open fade too.
hl.layer_rule({ name = "eww-search-blur", match = { namespace = "^eww-search$" }, blur = true, ignore_alpha = 0.2, no_anim = true })
