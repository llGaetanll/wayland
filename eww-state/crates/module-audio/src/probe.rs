//! Reading the sink volume out of wpctl, and the sink list out of pactl.

use crate::event::EventAudio;
use crate::model::Audio;
use crate::model::AudioOutputs;
use crate::model::AudioSink;
use crate::model::SinkKind;

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

/// The outputs, from pactl rather than wpctl.
///
/// `wpctl status` prints the same sinks but none of what makes the list usable:
/// no `node.nick`, no `device.icon_name`, and above all no port availability,
/// which is the only thing that distinguishes an HDMI port with a television on
/// the end from the four beside it that are merely wired to the chip.
pub fn sample_outputs() -> EventAudio {
    let default = exec::out("pactl", &["get-default-sink"]).unwrap_or_default();
    let listing = exec::out("pactl", &["list", "sinks"]).unwrap_or_default();
    EventAudio::OutputsObserved {
        outputs: AudioOutputs {
            sinks: parse_sinks(&listing),
            default,
        },
    }
}

/// Pull the usable sinks out of `pactl list sinks`.
///
/// Three fields per sink and one rule. `node.nick` is already the short name a
/// person would use — "Speaker", "HDMI 1" — where `Description` is the codec's
/// marketing string ("Core Ultra Processors (Series 3) HD Audio Speaker").
/// `device.icon_name` gives the kind. And a sink whose active port reports
/// `not available` is a jack with nothing in it: every HDMI port on the machine
/// exists as a sink permanently, so without that test the list is five
/// televisions that were never plugged in. A port that cannot tell — the
/// built-in speaker says `availability unknown` — is kept, because it is always
/// there.
fn parse_sinks(listing: &str) -> Vec<AudioSink> {
    let mut sinks = Vec::new();
    // A leading newline so the first sink splits like every other one: pactl's
    // output starts with "Sink #" rather than a blank line.
    let listing = format!("\n{listing}");
    for chunk in listing.split("\nSink #").skip(1) {
        let id = field(chunk, "Name").unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        // A sink with no port at all — a null sink, a loopback — is not a jack
        // and has nothing to be unplugged from, so only an explicit "not
        // available" excludes anything.
        if let Some(port) = field(chunk, "Active Port") {
            if !port_available(chunk, port.trim_start_matches("[Out] ")) {
                continue;
            }
        }
        let kind = SinkKind::from_icon_name(&prop(chunk, "device.icon_name").unwrap_or_default());
        let name = prop(chunk, "node.nick")
            .or_else(|| field(chunk, "Description"))
            .unwrap_or_else(|| id.clone());
        sinks.push(AudioSink { id, name, kind });
    }
    sinks
}

/// A `Key: value` line, at any indent.
fn field(chunk: &str, key: &str) -> Option<String> {
    chunk.lines().find_map(|line| {
        line.trim_start()
            .strip_prefix(key)?
            .strip_prefix(": ")
            .map(|v| v.trim().to_string())
    })
}

/// A `key = "value"` property from the Properties block.
fn prop(chunk: &str, key: &str) -> Option<String> {
    chunk.lines().find_map(|line| {
        let value = line.trim_start().strip_prefix(key)?.strip_prefix(" = ")?;
        let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// Whether the named port has something plugged into it.
///
/// The line reads `[Out] HDMI1: HDMI / DisplayPort 1 Output (type: HDMI,
/// priority: 500, availability group: HDMI/DP,pcm=5, not available)`. Only an
/// explicit "not available" excludes it: "availability unknown" is a port that
/// has no way to tell, which is every built-in speaker.
fn port_available(chunk: &str, port: &str) -> bool {
    for line in chunk.lines() {
        let line = line.trim_start();
        let Some(rest) = line.strip_prefix("[Out] ") else {
            continue;
        };
        let Some((name, detail)) = rest.split_once(": ") else {
            continue;
        };
        if name == port {
            return !detail.contains("not available");
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::parse_sinks;
    use crate::model::SinkKind;

    #[test]
    fn volume_and_mute() {
        let a = parse("Volume: 0.35");
        assert_eq!((a.volume, a.muted), (35, false));
        let a = parse("Volume: 0.60 [MUTED]");
        assert_eq!((a.volume, a.muted), (60, true));
        assert_eq!(parse("").volume, 0);
    }

    /// Trimmed from this machine: a speaker that cannot report availability, a
    /// headphone jack with nothing in it, and an HDMI port likewise.
    const LISTING: &str = r#"Sink #61
	Name: alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Speaker__sink
	Description: Core Ultra Processors (Series 3) HD Audio Speaker
	Properties:
		device.icon_name = "audio-speakers"
		node.nick = "Speaker"
	Ports:
		[Out] Speaker: Speaker (type: Speaker, priority: 100, availability unknown)
	Active Port: [Out] Speaker
Sink #62
	Name: alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Headphones__sink
	Description: Core Ultra Processors (Series 3) HD Audio Headphones
	Properties:
		device.icon_name = "audio-headphones"
		node.nick = "Headphones"
	Ports:
		[Out] Headphones: Headphones (type: Headphones, priority: 200, availability group: cs42l45 OT 43 Headphone, not available)
	Active Port: [Out] Headphones
Sink #63
	Name: alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__HDMI1__sink
	Description: Core Ultra Processors (Series 3) HD Audio HDMI / DisplayPort 1 Output
	Properties:
		device.icon_name = "video-display"
		node.nick = "HDMI 1"
	Ports:
		[Out] HDMI1: HDMI / DisplayPort 1 Output (type: HDMI, priority: 500, availability group: HDMI/DP,pcm=5, not available)
	Active Port: [Out] HDMI1
"#;

    #[test]
    fn an_empty_jack_is_not_an_output() {
        let sinks = parse_sinks(LISTING);
        assert_eq!(sinks.len(), 1, "only the speaker is plugged into anything");
        assert_eq!(sinks[0].name, "Speaker");
        assert_eq!(sinks[0].kind, SinkKind::Speaker);
    }

    #[test]
    fn a_plugged_port_joins_the_list() {
        let plugged = LISTING.replace(
            "availability group: cs42l45 OT 43 Headphone, not available",
            "availability group: cs42l45 OT 43 Headphone, available",
        );
        let sinks = parse_sinks(&plugged);
        assert_eq!(sinks.len(), 2);
        assert_eq!(sinks[1].name, "Headphones");
        assert_eq!(sinks[1].kind, SinkKind::Headphones);
    }

    #[test]
    fn the_nick_is_preferred_to_the_codecs_own_description() {
        let sinks = parse_sinks(LISTING);
        assert_eq!(
            sinks[0].name, "Speaker",
            "node.nick, not 'Core Ultra Processors (Series 3) HD Audio Speaker'"
        );
    }

    #[test]
    fn a_sink_with_no_nick_falls_back_to_its_description() {
        let no_nick = LISTING.replace("\t\tnode.nick = \"Speaker\"\n", "");
        let sinks = parse_sinks(&no_nick);
        assert_eq!(
            sinks[0].name,
            "Core Ultra Processors (Series 3) HD Audio Speaker"
        );
    }

    /// A null sink has no ports to report on. It is still somewhere sound can
    /// go, so the availability rule must not quietly swallow it.
    #[test]
    fn a_sink_with_no_ports_is_still_an_output() {
        let listing = "Sink #99\n\tName: null\n\tDescription: Null Output\n\tProperties:\n\t\tnode.nick = \"Null Output\"\n";
        let sinks = parse_sinks(listing);
        assert_eq!(sinks.len(), 1);
        assert_eq!(sinks[0].name, "Null Output");
    }

    #[test]
    fn nothing_at_all_is_no_outputs() {
        assert!(parse_sinks("").is_empty());
    }
}
