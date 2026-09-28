//! What this module has to be told rather than work out.

use crate::model::WorkspaceId;

#[derive(Clone, Debug)]
pub struct ConfigSpaces {
    /// The workspace minimized windows are kept on. A *special* workspace,
    /// which the compositor never shows unless it is asked to — which is the
    /// whole mechanism: a minimized window is not drawn, not in the switcher,
    /// and still a live client with its buffer intact.
    pub hidden: String,
    /// Where a directory of staged window thumbnails lives.
    pub thumbs: String,
    /// How tall a dock tile is, in screen pixels. macOS scales a minimized
    /// window to the tile's height and lets its width follow the window's
    /// aspect ratio, so the tiles are wide rectangles rather than squares —
    /// which means this module has to know the height to work out the width.
    pub tile_height: u32,
    /// Where a window found on the hidden workspace with nobody remembering it
    /// is sent. It has no recorded home — a previous daemon knew and died —
    /// and leaving it hidden would strand it for the rest of the session.
    pub fallback_home: WorkspaceId,
}

impl Default for ConfigSpaces {
    fn default() -> Self {
        ConfigSpaces {
            hidden: "special:minimized".to_string(),
            thumbs: "/tmp/eww-state-minimized".to_string(),
            tile_height: 34,
            fallback_home: WorkspaceId(1),
        }
    }
}
