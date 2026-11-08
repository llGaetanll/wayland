//! What the battery module can ask the outside world to do.

use api::JobId;

use crate::model::Battery;
use crate::model::HistPoint;

/// Performing an action is pure IO: each variant is handed everything it needs
/// and never reaches back into the state that asked for it.
#[derive(Debug)]
pub enum ActionBattery {
    /// Read the pack, and fold the reading into the on-disk ring.
    Sample {
        history: String,
        /// How far back the chart goes, in seconds. Older points are dropped on
        /// the way past, so the file cannot grow without bound.
        span: u64,
        /// Shortest gap between two stored points. The pack is sampled far more
        /// often than the chart has pixels, and a point every fifteen seconds
        /// would be ten thousand lines for one screen's worth of line.
        every: u64,
    },
    /// Time every process over a window and rank them.
    SampleProcs {
        window_ms: u32,
        top: usize,
        /// Dock ids and their icons, so a row can be given the app's own
        /// picture. Handed down rather than looked up: this module does not
        /// know there is a dock.
        icons: Vec<(String, String)>,
        /// Which process owns which window, as `(pid, class)`. Handed down for
        /// the same reason: this module does not know there is a compositor,
        /// only that some processes belong to something with a name.
        owners: Vec<(u32, String)>,
        fallback_icon: String,
    },
    /// Draw the pill for exactly this state.
    Render {
        id: JobId,
        battery: Battery,
        dark: bool,
    },
    /// Draw the chart for exactly these points.
    RenderGraph {
        id: JobId,
        points: Vec<HistPoint>,
        span: u64,
        dark: bool,
    },
}
