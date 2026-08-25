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
    }
}
