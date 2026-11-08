//! Everything that can move the launcher forward.

use api::JobId;

use crate::model::App;

#[derive(Debug)]
pub enum EventSearch {
    /// The launcher keybind was pressed. A toggle: pressing it again while the
    /// panel is up puts it away, the way the bar dropdowns work.
    Toggled,
    /// Put the panel away without launching anything — Escape, or a click
    /// outside it.
    Dismissed,
    /// The text field changed. Carries the whole field, not a keystroke: eww
    /// hands us the entry's contents on every change, and a field is smaller
    /// than the history of how it came to say that.
    Queried {
        text: String,
    },
    /// Move the highlight. Negative is up.
    Moved {
        delta: i32,
    },
    /// Return was pressed. It carries the field as well, because eww runs each
    /// widget handler on a thread of its own: the change for the last letter
    /// typed and the Return that follows it race, and a Return that brought no
    /// text would launch whatever the previous letter had selected.
    Accepted {
        text: String,
    },
    /// A result was clicked. Its command rather than its position, because the
    /// list may have been rebuilt between the draw and the click.
    Picked {
        exec: String,
    },
    /// The installed applications, as the last scan found them.
    Indexed {
        apps: Vec<App>,
    },
    Finished {
        id: JobId,
    },
}
