-- The Lua config has a permission model; without this, plugins are denied.
hl.permission("/usr/(bin|local/bin)/hyprpm", "plugin", "allow")
hl.permission("/home/al/files/github/hyprland-plugins/.*", "plugin", "allow")
hl.permission("/home/al/files/github/blur/.*", "plugin", "allow")

require "plugins.hyprbars"
