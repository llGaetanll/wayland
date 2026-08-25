//! Carrying out a file action.

use eww_state_core::ActionFile;

use crate::probe_dock;
use crate::proc;
use crate::runtime_daemon::RuntimeDaemon;
use crate::svg;

impl RuntimeDaemon {
    pub(crate) fn perform_action_file(&self, action: ActionFile) -> (bool, Option<String>) {
        let paths = self.paths();
        let ok = match action {
            ActionFile::WriteDock { apps } => probe_dock::write(paths, &apps).is_ok(),
            ActionFile::RenderBattery { percent, charging } => {
                svg::render_battery(&paths.cache, percent, charging).is_ok()
            }
            ActionFile::SaveShot { path } => {
                let name = proc::out("date", &["+Screenshot %Y-%m-%d %H.%M.%S.png"])
                    .unwrap_or_else(|| "Screenshot.png".into());
                std::fs::copy(&path, paths.screenshots.join(name)).is_ok()
            }
            ActionFile::DeleteShot { path } => std::fs::remove_file(&path).is_ok(),
        };
        (ok, None)
    }
}
