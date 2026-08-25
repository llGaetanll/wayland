//! Drawing the pill.
//!
//! The filename encodes exactly what the drawing depends on, so a changed
//! picture always means a changed path — which is what makes eww and GTK reload
//! it, since they cache by path and new bytes at the same path would be missed.

use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::model::Battery;

/// Where the pill for this exact state lives. Pure: the state machine names the
/// file it wants to point at without anything having been written yet.
pub fn path(cache_dir: &str, battery: Battery) -> String {
    format!(
        "{cache_dir}/battery-{}-{}.svg",
        battery.percent,
        u8::from(battery.charging)
    )
}

pub fn render(cache: &Path, battery: Battery) -> io::Result<PathBuf> {
    let (percent, charging) = (battery.percent, battery.charging);
    let out = PathBuf::from(path(&cache.to_string_lossy(), battery));
    // Prune the previous variants so the cache holds only the current pill.
    if let Ok(entries) = std::fs::read_dir(cache) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("battery-") && name.ends_with(".svg") && e.path() != out {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    if out.exists() {
        return Ok(out);
    }

    let fill = if charging {
        "#30D158"
    } else if percent <= 20 {
        "#FF453A"
    } else {
        "#FFFFFF"
    };
    // Full charge needs a smaller number to fit three digits in the pill.
    let font_size = if percent >= 100 { 9 } else { 12 };
    let fill_width = 18.4 * percent as f64 / 100.0;

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 25 17">
  <rect x="0.5" y="0.5" width="21" height="16" rx="4" fill="#4a4a4c"/>
  <rect x="22" y="5.5" width="2.2" height="6" rx="1.1" fill="#4a4a4c"/>
  <rect x="1.8" y="1.8" width="{fill_width:.2}" height="13.4" rx="2.6" fill="{fill}"/>
  <text x="11" y="12.4" font-family="Inter, sans-serif" font-size="{font_size}"
        font-weight="700" text-anchor="middle"
        fill="#000000" stroke="#ffffff" stroke-width="1.2"
        paint-order="stroke" style="paint-order:stroke">{percent}</text>
</svg>
"##
    );
    std::fs::write(&out, svg)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(percent: u8, charging: bool) -> Battery {
        Battery { percent, charging }
    }

    #[test]
    fn the_path_encodes_the_drawing() {
        assert_ne!(
            path("/c", level(55, false)),
            path("/c", level(55, true)),
            "plugging in must change the path or the icon would not reload"
        );
        assert_ne!(path("/c", level(55, false)), path("/c", level(56, false)));
    }

    #[test]
    fn renders_once_and_prunes_the_rest() {
        let dir = std::env::temp_dir().join(format!("battery-svg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let a = render(&dir, level(40, false)).expect("rendered");
        assert!(a.exists());
        let b = render(&dir, level(41, false)).expect("rendered");
        assert!(b.exists());
        assert!(!a.exists(), "the stale variant is cleaned up");
        std::fs::remove_dir_all(&dir).ok();
    }
}
