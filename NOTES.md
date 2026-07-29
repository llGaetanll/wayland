File to keep track of bugs, features, or general notes. Each bullet is annotated
with (h/m/l) to indicate priority.

# Misc
- [ ] (m) Clean up hyperland.lua files into cleaner structure. Ease off on prolific
  comments
- [ ] (m) Might want to subdivide ~/.config/eww/scripts directory into
  subfolders for each module. Cleaner and more maintainable

# Neovim
- [ ] (l) `.yuck` files still not syntax highlighted in buffers, even though
  grammar is installed in TreeSitter

# Top Bar

## Battery
- [ ] (l) Modify battery svg to match iPhone svg (currently incorrect)

## Bluetooth
- [ ] (l) Display currently connected-to devices
- [ ] (h) Module often will no close for no obvious reason

## WiFi
- [ ] (l) Display currently connected-to wifi

# Desktop Switcher
- [ ] (h) When multiple firefox windows are opened, if one is fullscreened, the
  other becomes unaccessible
- [ ] (m) Build multi-desktop preview

# Search
- [ ] (l) Make it possible to search files in rofi module
- [ ] (l) As the user types into the searchbar, fewer and fewer results are
  displayed. Since the entire module is vertically centered on the screen, this
  moves the search down as we type. This is bad UX. We want the seachbar to be
  fixed in place.
- [ ] (l) Make rofi module telescope-nvim-like in layout: Search and results on
  the left, preview window on the right. This also matches the mac-os experience
  more closely.


