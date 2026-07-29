File to keep track of bugs, features, or general notes. Each bullet is annotated
with (h/m/l) to indicate priority.

# Misc
- [ ] (h) Define a shared color system used consistently across eww widgets and
  rest of the system. Only two colors for now: `background`, and `accent`.
  `background` should be the current background color used by subwidgets in the
  top bar. `accent` should be an appropriate shade blue.
- [ ] (m) Clean up hyperland.lua files into cleaner structure. Ease off on prolific
  comments
- [ ] (m) Might want to subdivide ~/.config/eww/scripts directory into
  subfolders for each module. Cleaner and more maintainable
- [ ] (m) Add minimize button to title bar. Minimized window should go into the
  dock, similar to mac os
- [ ] (m) The volume/brightness sliders are two-toned: the color of the slider,
  and the color of the box containing the slider. The color of the box
  containing the slider is too transparent. The color of the box containing the
  slider should be the background color used by the sub-widgets, and the color
  of the slider should be the accent color.

# Neovim
- [ ] (h) `.yuck` files still not syntax highlighted in buffers, even though
  grammar is installed in TreeSitter

# Top Bar

- [ ] (h) Clicking outside of a sub-widget should hide that subwidget
- [ ] (l) Pressing ESC when a sub-widget is shown should high it

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


