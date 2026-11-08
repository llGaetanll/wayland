//! Carrying out what the screenshot module asked for.
//!
//! The capture is the interesting one, and it is a workaround: the whole screen
//! is grabbed *before* the selector draws, then cropped to the chosen region.
//! `grim -g "$(slurp)"` races — grim can grab a frame in which the compositor
//! has not yet torn down slurp's overlay, baking the blue selection into the
//! shot. Grabbing first makes that impossible.

use std::path::Path;
use std::path::PathBuf;

use crate::action::ActionShot;
use crate::event::EventShot;
use crate::model::Shot;

/// Where captures are staged and saved, and how the selector is themed. The
/// module knows none of this; the runtime hands it in.
pub struct Places {
    pub staging: PathBuf,
    pub saved: PathBuf,
    /// Accent colour for slurp, without its leading `#`.
    pub accent: String,
    /// `date(1)` format for a saved file's name.
    pub saved_format: String,
}

/// Carrying out a capture produces two events: the file it staged, and the job
/// finishing. Everything else produces at most one.
pub fn perform(action: &ActionShot, places: &Places) -> Vec<EventShot> {
    match action {
        ActionShot::Capture { id } => {
            let mut out = Vec::new();
            if let Some(shot) = capture(places) {
                out.push(EventShot::Staged { shot });
            }
            out.push(EventShot::Finished { id: *id });
            out
        }
        ActionShot::Copy { path } => {
            let _ = exec::pipe_file("wl-copy", &["--type", "image/png"], path);
            Vec::new()
        }
        ActionShot::Save { path } => {
            let name = exec::out("date", &[&places.saved_format])
                .unwrap_or_else(|| "Screenshot.png".into());
            // The directory is made once at startup, which is not a promise
            // that it is still there: losing a shot to a renamed folder is the
            // one outcome this button must never have.
            let _ = std::fs::create_dir_all(&places.saved);
            if let Err(e) = std::fs::copy(path, places.saved.join(name)) {
                tracing::error!(error = %e, "could not save the screenshot");
            }
            Vec::new()
        }
        ActionShot::Delete { path } => {
            let _ = std::fs::remove_file(path);
            Vec::new()
        }
    }
}

fn capture(places: &Places) -> Option<Shot> {
    let dir = &places.staging;
    let _ = std::fs::create_dir_all(dir);
    reap(dir);

    let stamp = exec::out("date", &["+%s%N"]).unwrap_or_default();
    let full = dir.join(format!("full-{stamp}.png"));
    let full_s = full.to_string_lossy().into_owned();
    if !exec::ok("grim", &[&full_s]) {
        return None;
    }

    let accent = &places.accent;
    let geom = exec::out(
        "slurp",
        &[
            "-b",
            "00000040",
            "-c",
            &format!("{accent}ff"),
            "-s",
            &format!("{accent}26"),
            "-w",
            "2",
        ],
    );
    let Some((x, y, width, height)) = geom.as_deref().and_then(parse_geometry) else {
        let _ = std::fs::remove_file(&full);
        return None;
    };

    let shot = dir.join(format!("shot-{stamp}.png"));
    let shot_s = shot.to_string_lossy().into_owned();
    let cropped = exec::ok(
        "magick",
        &[
            &full_s,
            "-crop",
            &format!("{width}x{height}+{x}+{y}"),
            "+repage",
            &shot_s,
        ],
    );
    let _ = std::fs::remove_file(&full);
    if !cropped {
        return None;
    }
    Some(Shot {
        path: shot_s,
        width,
        height,
    })
}

/// Drop shots left over from previous runs; none of this run's files exist yet.
fn reap(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.path().extension().is_some_and(|x| x == "png") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// slurp prints `X,Y WxH`.
fn parse_geometry(geom: &str) -> Option<(u32, u32, u32, u32)> {
    let (xy, wh) = geom.trim().split_once(' ')?;
    let (x, y) = xy.split_once(',')?;
    let (w, h) = wh.split_once('x')?;
    Some((
        x.parse().ok()?,
        y.parse().ok()?,
        w.parse().ok()?,
        h.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_geometry;
    use super::perform;
    use super::Places;
    use crate::action::ActionShot;

    #[test]
    fn saving_works_when_the_destination_is_gone() {
        let dir = std::env::temp_dir().join("eww-state-shot-save");
        let _ = std::fs::remove_dir_all(&dir);
        let staged = dir.join("staged.png");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&staged, b"not really a png").unwrap();

        // The directory the daemon made at startup can be renamed or removed
        // while it runs; a shot must not be lost because of that.
        let saved = dir.join("gone/Screenshots");
        let places = Places {
            staging: dir.clone(),
            saved: saved.clone(),
            accent: "0a84ff".into(),
            saved_format: "+Screenshot.png".into(),
        };
        perform(
            &ActionShot::Save {
                path: staged.to_string_lossy().into_owned(),
            },
            &places,
        );

        assert!(saved.join("Screenshot.png").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn slurp_geometry() {
        assert_eq!(
            parse_geometry("100,200 640x480"),
            Some((100, 200, 640, 480))
        );
        assert_eq!(parse_geometry(""), None);
        assert_eq!(parse_geometry("garbage"), None);
    }
}
