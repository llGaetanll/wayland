//! Drawing the chart.
//!
//! Same contract as the pill in [`crate::svg`]: the filename encodes the
//! picture, so a changed chart always means a changed path — which is what
//! makes eww and GTK reload it, since both cache by path and new bytes at the
//! same path would be missed.

use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::model::HistPoint;

/// The chart's own coordinate space, in points. It is the panel's 298pt content
/// width and the height the stylesheet gives `.batt-graph`; the SVG is authored
/// at exactly that size so `background-size: contain` never scales it.
const W: f64 = 338.0;
const H: f64 = 86.0;

/// Where the chart for exactly this picture lives. Pure: the state machine can
/// name the file it wants to point at before anything has been written.
pub fn path(cache_dir: &str, key: u64) -> String {
    format!("{cache_dir}/battery-graph-{key:016x}.svg")
}

/// A fingerprint of everything the drawing depends on.
///
/// FNV-1a, because this needs to be stable across runs and a `DefaultHasher` is
/// explicitly not: its seed may change between releases, which would orphan
/// every chart in the cache on a rustc upgrade.
pub fn key(points: &[HistPoint], span: u64, dark: bool) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut byte = |b: u8| {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    };
    for p in points {
        for b in p.at.to_le_bytes() {
            byte(b);
        }
        byte(p.percent);
        byte(u8::from(p.charging));
    }
    for b in span.to_le_bytes() {
        byte(b);
    }
    byte(u8::from(dark));
    h
}

pub fn render(cache: &Path, points: &[HistPoint], span: u64, dark: bool) -> io::Result<PathBuf> {
    let dir = cache.to_string_lossy().to_string();
    let out = PathBuf::from(path(&dir, key(points, span, dark)));
    // Only the current chart is worth keeping: the ring moves every minute, so
    // without this the cache would collect one SVG per sample for ever.
    if let Ok(entries) = std::fs::read_dir(cache) {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("battery-graph-") && name.ends_with(".svg") && e.path() != out {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    if out.exists() {
        return Ok(out);
    }
    std::fs::write(&out, svg(points, span, dark))?;
    Ok(out)
}

fn svg(points: &[HistPoint], span: u64, dark: bool) -> String {
    // The right edge is always now and the left is always `span` ago, whatever
    // the ring happens to hold. A fresh ring therefore draws a short line at
    // the right rather than a full-width one — which is the truth, and the
    // alternative would have the section label lying about the time axis.
    let now = points.last().map_or(0, |p| p.at);
    let start = now.saturating_sub(span);
    let x = |at: u64| (at.saturating_sub(start) as f64) / (span.max(1) as f64) * W;
    let y = |percent: u8| (H - 1.0) - f64::from(percent) / 100.0 * (H - 2.0);

    let grid = if dark { "#ffffff" } else { "#000000" };
    let grid_opacity = if dark { 0.10 } else { 0.08 };
    let bands = charging_bands(points, &x, dark);

    let mut line = String::new();
    for (i, p) in points.iter().enumerate() {
        line.push_str(&format!(
            "{}{:.2},{:.2} ",
            if i == 0 { "M" } else { "L" },
            x(p.at),
            y(p.percent)
        ));
    }
    // One point is not a line. SVG draws nothing for a lone moveto, which is
    // the right answer for a ring with a single sample in it.
    let area = match (points.first(), points.last()) {
        (Some(first), Some(last)) if points.len() > 1 => format!(
            r#"<path d="{line}L{:.2},{H} L{:.2},{H} Z" fill="url(#f)"/>"#,
            x(last.at),
            x(first.at)
        ),
        _ => String::new(),
    };
    let stroke = if points.len() > 1 {
        format!(
            r##"<path d="{line}" fill="none" stroke="#0a84ff" stroke-width="1.6"
        stroke-linejoin="round" stroke-linecap="round"/>"##
        )
    } else {
        String::new()
    };

    // Four times the viewBox, like the pill and for the same reason: GTK3 loads
    // a CSS background-image SVG through GdkPixbuf, which rasterizes at the
    // INTRINSIC size and scales the bitmap to the box. Rasterizing large and
    // scaling down is free and stays crisp.
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{:.0}" viewBox="0 0 {W:.0} {H:.0}">
  <defs>
    <linearGradient id="f" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#0a84ff" stop-opacity="0.22"/>
      <stop offset="1" stop-color="#0a84ff" stop-opacity="0.00"/>
    </linearGradient>
  </defs>
  {bands}
  <g stroke="{grid}" stroke-opacity="{grid_opacity}" stroke-width="1">
    <line x1="0" y1="{:.2}" x2="{W:.0}" y2="{:.2}"/>
    <line x1="0" y1="{:.2}" x2="{W:.0}" y2="{:.2}"/>
  </g>
  {area}
  {stroke}
</svg>
"##,
        W * 4.0,
        H * 4.0,
        y(50),
        y(50),
        y(0),
        y(0),
    )
}

/// The stretches spent on the cable, shaded behind the line.
///
/// Worth drawing because it is the answer to the question the chart provokes:
/// a climb is not the battery recovering on its own, it is where the laptop was
/// plugged in, and without the band the two readings look the same.
fn charging_bands(points: &[HistPoint], x: &impl Fn(u64) -> f64, dark: bool) -> String {
    let fill = if dark { "#ffffff" } else { "#000000" };
    let opacity = if dark { 0.06 } else { 0.05 };
    let mut out = String::new();
    let mut from: Option<u64> = None;
    for p in points {
        match (p.charging, from) {
            (true, None) => from = Some(p.at),
            (false, Some(start)) => {
                out.push_str(&band(x(start), x(p.at), fill, opacity));
                from = None;
            }
            _ => {}
        }
    }
    // Still on the cable at the right-hand edge.
    if let (Some(start), Some(last)) = (from, points.last()) {
        out.push_str(&band(x(start), x(last.at), fill, opacity));
    }
    out
}

fn band(x0: f64, x1: f64, fill: &str, opacity: f64) -> String {
    format!(
        r#"<rect x="{x0:.2}" y="0" width="{:.2}" height="{H:.0}" fill="{fill}" opacity="{opacity}"/>"#,
        (x1 - x0).max(0.0)
    )
}
