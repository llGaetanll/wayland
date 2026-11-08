//! Carrying out what the launcher asked for.

use std::path::PathBuf;

use crate::action::ActionSearch;
use crate::config::ConfigSearch;
use crate::event::EventSearch;

/// Where the applications and their icons live. The module knows none of this;
/// the runtime hands it in.
pub struct Places {
    /// `.desktop` directories, in precedence order — the user's own first.
    pub app_dirs: Vec<PathBuf>,
    /// Icon themes to search, in preference order.
    pub icon_dirs: Vec<PathBuf>,
    /// Where rendered SVGs are kept.
    pub cache: PathBuf,
}

/// The Hyprland submap that routes Escape and the arrow keys here.
///
/// KEEP IN SYNC with `hl.define_submap` in ~/.config/hypr/hyprland.lua. While
/// it is active, the keys it names come to us and every other key still reaches
/// the text field, which is what lets the user keep typing.
const SUBMAP: &str = "search";

/// Indexing produces two events: what it found, and the job finishing.
/// Everything else produces none.
pub fn perform(action: &ActionSearch, places: &Places, cfg: &ConfigSearch) -> Vec<EventSearch> {
    match action {
        ActionSearch::Index { id } => vec![
            EventSearch::Indexed {
                apps: crate::probe::index(places, &cfg.terminal),
            },
            EventSearch::Finished { id: *id },
        ],
        ActionSearch::Launch { exec } => {
            exec::detach(exec);
            Vec::new()
        }
        ActionSearch::Keys { grabbed } => {
            let to = if *grabbed { SUBMAP } else { "reset" };
            // This compositor is configured in Lua, so a dispatch is a Lua
            // expression rather than a dispatcher name. See module-spaces.
            if !exec::ok("hyprctl", &["dispatch", &format!("hl.dsp.submap('{to}')")]) {
                tracing::warn!(submap = to, "the compositor refused a submap change");
            }
            Vec::new()
        }
    }
}
