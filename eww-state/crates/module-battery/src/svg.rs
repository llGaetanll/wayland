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
pub fn path(cache_dir: &str, battery: Battery, dark: bool) -> String {
    format!(
        "{cache_dir}/battery-{}-{}-{}.svg",
        battery.percent,
        u8::from(battery.charging),
        if dark { "d" } else { "l" }
    )
}

pub fn render(cache: &Path, battery: Battery, dark: bool) -> io::Result<PathBuf> {
    let (percent, charging) = (battery.percent, battery.charging);
    let out = PathBuf::from(path(&cache.to_string_lossy(), battery, dark));
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

    // macOS draws the battery as an outlined capsule with a solid level inside
    // and a bolt struck through it while it charges. Every number below was
    // fitted, not guessed: `macos-match/tools/sweep_batt.py` renders a grid of
    // candidates and scores each one's ink mask against the glyph cropped out of
    // menubar-full.png. This set scores 85.6% IoU; the whole glyph is 25.5 x 12
    // points, which is what the viewBox is in.
    //
    // The width and height attributes are load-bearing, not decoration. With
    // only a viewBox the SVG has no intrinsic size and GTK's
    // `background-size: contain` draws the glyph several points too small. And
    // they are 4x the viewBox on purpose: GTK3 loads a CSS background-image SVG
    // through GdkPixbuf, which rasterizes at the INTRINSIC size and then scales
    // the bitmap to the box, so an intrinsic 25.5 in a 32px box is an upscale
    // and the glyph comes out soft. Rasterizing large and scaling down is free
    // and stays crisp.
    //
    // The two opacities are measured, not stylistic. Reading the macOS glyph
    // column by column at its right-hand end: the outline peaks at luminance
    // 119 and the nub at 141, against a 231 fill — so 0.5 and 0.6, not the 0.4
    // both started at. The nub is also 2 points wide and sits at x 24..25.5,
    // which is what carries the glyph out to its full 25.5.
    // Not pure white: every glyph in menubar-full.png peaks at luminance 231,
    // which is macOS's dark-mode label colour, and #E7E7E7 is exactly that.
    // macOS's menu bar label colour for the current appearance: luminance 231
    // on a dark bar, near-black on a light one. Low-battery red is the same in
    // both, as it is in macOS.
    let ink = if dark { "#E7E7E7" } else { "#1A1A1A" };
    let fill = if !charging && percent <= 20 {
        "#FF453A"
    } else {
        ink
    };
    // The level runs from x=1.9 to x=20.9 inside the 1pt stroke, so a full
    // battery still reads as a fill within an outline and not one solid block.
    let level = 19.0 * (percent.min(100) as f64) / 100.0;
    // Never draw a level so short it reads as a rendering artefact.
    let level = if percent > 0 { level.max(1.5) } else { 0.0 };
    // White bolt over a dark outline drawn first (paint-order), which is how
    // macOS keeps it legible both against a white level and against the menu
    // bar showing through an empty one.
    // The bolt is knocked out against whatever the bar is, so its outline is the
    // bar's colour, not a fixed dark.
    let knockout = if dark { "#0b0b0d" } else { "#f2f2f4" };
    let bolt = if charging {
        format!(
            r##"<path d="M 13.44 0.15 L 7.05 6.85 L 10.68 6.85 L 9.06 11.85 L 15.45 5.15 L 11.83 5.15 Z"
        fill="{ink}" stroke="{knockout}" stroke-width="2.0" stroke-linejoin="round"
        paint-order="stroke" style="paint-order:stroke"/>"##
        )
    } else {
        String::new()
    };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="102" height="48" viewBox="0 0 25.5 12">
  <rect x="0.5" y="0.5" width="21.45" height="11" rx="3.4"
        fill="none" stroke="{ink}" stroke-opacity="0.5" stroke-width="1.0"/>
  <rect x="23.9" y="4" width="1.6" height="4" rx="0.8"
        fill="#E7E7E7" fill-opacity="0.6"/>
  <rect x="1.9" y="2.0" width="{level:.2}" height="8.0" rx="1.6" fill="{fill}"/>
  {bolt}
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
            path("/c", level(55, false), true),
            path("/c", level(55, true), true),
            "plugging in must change the path or the icon would not reload"
        );
        assert_ne!(
            path("/c", level(55, false), true),
            path("/c", level(56, false), true)
        );
        assert_ne!(
            path("/c", level(55, false), true),
            path("/c", level(55, false), false),
            "light and dark are different drawings, so they need different paths"
        );
    }

    #[test]
    fn renders_once_and_prunes_the_rest() {
        let dir = std::env::temp_dir().join(format!("battery-svg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let a = render(&dir, level(40, false), true).expect("rendered");
        assert!(a.exists());
        let b = render(&dir, level(41, false), true).expect("rendered");
        assert!(b.exists());
        assert!(!a.exists(), "the stale variant is cleaned up");
        std::fs::remove_dir_all(&dir).ok();
    }
}
