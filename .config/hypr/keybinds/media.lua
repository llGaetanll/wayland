local mod = require("config.vars").mod

-- locked = keeps working over the lockscreen, repeating = hold to ramp.
local locked = { locked = true }
local ramp = { locked = true, repeating = true }

-- -l 1 on the raise caps the sink at 100%; lowering needs no cap.
local vol_up     = "wpctl set-volume -l 1 @DEFAULT_AUDIO_SINK@ 5%+"
local vol_down   = "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-"
local bright_up   = "brightnessctl -e4 -n2 set 5%+"
local bright_down = "brightnessctl -e4 -n2 set 5%-"

-- The keyboard backlight is a 3-step LED (0/1/2) that cycles on one key.
-- Writing it needs the `input` group; without that these are no-ops.
local kbd_up   = "brightnessctl -d dell::kbd_backlight set +1"
local kbd_down = "brightnessctl -d dell::kbd_backlight set 1-"

return {
    -- Volume: SUPER +/-  (plus and equal so shift does not matter)
    { key = mod .. " + equal", action = hl.dsp.exec_cmd(vol_up),   opts = ramp },
    { key = mod .. " + plus",  action = hl.dsp.exec_cmd(vol_up),   opts = ramp },
    { key = mod .. " + minus", action = hl.dsp.exec_cmd(vol_down), opts = ramp },

    -- Brightness: SUPER + ALT +/-  (needs brightnessctl)
    { key = mod .. " + ALT + equal", action = hl.dsp.exec_cmd(bright_up),   opts = ramp },
    { key = mod .. " + ALT + plus",  action = hl.dsp.exec_cmd(bright_up),   opts = ramp },
    { key = mod .. " + ALT + minus", action = hl.dsp.exec_cmd(bright_down), opts = ramp },

    -- Laptop function row. The keys arrive as ordinary XF86 keysyms on the
    -- hid-sdw:...-consumer-control keyboard, so they are bound by keysym and the
    -- physical order of the row does not matter.
    { key = "XF86AudioRaiseVolume", action = hl.dsp.exec_cmd(vol_up),   opts = ramp },
    { key = "XF86AudioLowerVolume", action = hl.dsp.exec_cmd(vol_down), opts = ramp },
    { key = "XF86AudioMute",        action = hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), opts = locked },

    -- The Dell privacy driver already cuts the mic in hardware; this keeps the
    -- PipeWire source in step so apps see the same state.
    { key = "XF86AudioMicMute", action = hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle"), opts = locked },

    { key = "XF86MonBrightnessUp",   action = hl.dsp.exec_cmd(bright_up),   opts = ramp },
    { key = "XF86MonBrightnessDown", action = hl.dsp.exec_cmd(bright_down), opts = ramp },

    { key = "XF86KbdBrightnessUp",   action = hl.dsp.exec_cmd(kbd_up),   opts = locked },
    { key = "XF86KbdBrightnessDown", action = hl.dsp.exec_cmd(kbd_down), opts = locked },
    { key = "XF86KbdLightOnOff",     action = hl.dsp.exec_cmd("sh -c '" .. kbd_up .. " || brightnessctl -d dell::kbd_backlight set 0'"), opts = locked },

    -- Playback keys go to whichever player has the MPRIS focus.
    { key = "XF86AudioPlay",  action = hl.dsp.exec_cmd("playerctl play-pause"), opts = locked },
    { key = "XF86AudioPause", action = hl.dsp.exec_cmd("playerctl play-pause"), opts = locked },
    { key = "XF86AudioStop",  action = hl.dsp.exec_cmd("playerctl stop"),       opts = locked },
    { key = "XF86AudioNext",  action = hl.dsp.exec_cmd("playerctl next"),       opts = locked },
    { key = "XF86AudioPrev",  action = hl.dsp.exec_cmd("playerctl previous"),   opts = locked },
}
