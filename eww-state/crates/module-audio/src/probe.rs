//! Reading the sink volume out of wpctl.

use crate::event::EventAudio;
use crate::model::Audio;

pub fn sample() -> Option<EventAudio> {
    let out = exec::out("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
    Some(EventAudio::Observed { audio: parse(&out) })
}

/// `wpctl get-volume` prints `Volume: 0.35` and appends `[MUTED]` when muted.
fn parse(out: &str) -> Audio {
    let level = out
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);
    Audio {
        volume: (level * 100.0).round().clamp(0.0, 100.0) as u8,
        muted: out.contains("MUTED"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn volume_and_mute() {
        let a = parse("Volume: 0.35");
        assert_eq!((a.volume, a.muted), (35, false));
        let a = parse("Volume: 0.60 [MUTED]");
        assert_eq!((a.volume, a.muted), (60, true));
        assert_eq!(parse("").volume, 0);
    }
}
