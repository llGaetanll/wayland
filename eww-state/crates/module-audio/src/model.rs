//! What the world looks like, as far as volume is concerned.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Audio {
    pub volume: u8,
    pub muted: bool,
}

/// What kind of thing sound is coming out of. Only ever used to pick a glyph —
/// nothing decides anything on it, which is why a sink we cannot place falls
/// back to `Speaker` rather than carrying an "unknown" the widget would have to
/// have a case for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SinkKind {
    #[default]
    Speaker,
    Headphones,
    Display,
}

impl SinkKind {
    /// Read the kind off the device's own `device.icon_name`, which PipeWire
    /// fills in from the ALSA port type. Guessing from the sink name instead
    /// means parsing "HiFi__HDMI1__sink", which is a different string on every
    /// machine.
    pub fn from_icon_name(icon: &str) -> SinkKind {
        match icon {
            "audio-headphones" | "audio-headset" => SinkKind::Headphones,
            "video-display" => SinkKind::Display,
            _ => SinkKind::Speaker,
        }
    }
}

/// One thing sound can be sent to.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioSink {
    /// The sink name, which is what switching to it is addressed by.
    pub id: String,
    pub name: String,
    pub kind: SinkKind,
}

/// Every output that currently exists, and which one is in use.
///
/// `default` is held rather than a flag per sink: which one is active is one
/// fact, and a bool on each is the same fact written down five times, free to
/// disagree with itself.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AudioOutputs {
    pub sinks: Vec<AudioSink>,
    pub default: String,
}

impl AudioOutputs {
    pub fn active(&self) -> Option<&AudioSink> {
        self.sinks.iter().find(|s| s.id == self.default)
    }
}
