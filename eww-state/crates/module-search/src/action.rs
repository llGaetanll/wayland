//! What the launcher can ask the outside world to do.

use api::JobId;

#[derive(Debug)]
pub enum ActionSearch {
    /// Read every `.desktop` file and resolve its icon. Slow enough to be worth
    /// an id — a second scan while one is running would be pure waste.
    Index {
        id: JobId,
    },
    Launch {
        exec: String,
    },
    /// Route Escape and the arrow keys to us, or stop.
    ///
    /// eww has no key bindings of any kind, so the only thing that can see a
    /// keypress the text field does not want is the compositor. `grabbed` is a
    /// Hyprland submap: while it is on, the keys the submap names come here and
    /// everything else still reaches the field.
    Keys {
        grabbed: bool,
    },
}
