hl.on("hyprland.start", function() hl.exec_cmd("hyprpaper") end)

-- Audio stack (also required for Bluetooth audio)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire") end)
hl.on("hyprland.start", function() hl.exec_cmd("wireplumber") end)
hl.on("hyprland.start", function() hl.exec_cmd("pipewire-pulse") end)
