local modules = { "apps", "window", "media" }

for _, name in ipairs(modules) do
    for _, bind in ipairs(require("keybinds." .. name)) do
        hl.bind(bind.key, bind.action, bind.opts)
    end
end
