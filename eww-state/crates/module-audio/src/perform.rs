//! Carrying out what the volume module asked for.

use crate::action::ActionAudio;
use crate::event::EventAudio;
use crate::probe;

pub fn perform(action: &ActionAudio) -> Option<EventAudio> {
    match action {
        ActionAudio::Sample => probe::sample(),
        ActionAudio::Set { percent } => {
            exec::ok(
                "wpctl",
                &[
                    "set-volume",
                    "-l",
                    "1",
                    "@DEFAULT_AUDIO_SINK@",
                    &format!("{percent}%"),
                ],
            );
            Some(EventAudio::Finished)
        }
        ActionAudio::SampleOutputs => Some(probe::sample_outputs()),
        // Streams already playing stay where they are; pactl moves them so the
        // switch is audible immediately rather than at the next track.
        ActionAudio::SetDefaultOutput { id } => {
            let ok = exec::ok("pactl", &["set-default-sink", id]);
            if ok {
                move_streams(id);
            }
            Some(EventAudio::OutputSwitched { ok })
        }
    }
}

/// Send everything currently playing to the new sink.
///
/// Best effort, and deliberately not waited on: a stream that ends between the
/// listing and the move makes `move-sink-input` fail, which says nothing about
/// whether the switch worked.
fn move_streams(sink: &str) {
    let Some(listing) = exec::out("pactl", &["list", "short", "sink-inputs"]) else {
        return;
    };
    for input in listing.lines().filter_map(|l| l.split('\t').next()) {
        exec::ok("pactl", &["move-sink-input", input, sink]);
    }
}
