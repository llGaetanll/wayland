//! What the volume module does, said as the conversations it can have.
//!
//! Nothing here reaches inside `StateAudio`: a test asks the same questions the
//! bar asks — the actions that came out, and the vars that go to eww.

use api::Module;
use api::WireVars;
use eval::eval;
use module_audio::model::AudioOutputs;
use module_audio::model::AudioSink;
use module_audio::ActionAudio;
use module_audio::ConfigAudio;
use module_audio::EventAudio;
use module_audio::SinkKind;
use module_audio::StateAudio;

fn audio() -> StateAudio {
    StateAudio::new(ConfigAudio::default())
}

fn sink(id: &str, name: &str, kind: SinkKind) -> AudioSink {
    AudioSink {
        id: id.to_string(),
        name: name.to_string(),
        kind,
    }
}

/// A speaker and a pair of headphones, with sound going to the speaker.
fn seen(default: &str) -> EventAudio {
    EventAudio::OutputsObserved {
        outputs: AudioOutputs {
            sinks: vec![
                sink("speaker", "Speaker", SinkKind::Speaker),
                sink("phones", "AirPods Pro", SinkKind::Headphones),
            ],
            default: default.to_string(),
        },
    }
}

fn var(a: &StateAudio, name: &str) -> String {
    let mut vars = WireVars::default();
    a.vars(&mut vars);
    vars.get(name).unwrap_or_default().to_string()
}

#[test]
fn opening_the_panel_asks_for_both_the_level_and_the_list() {
    let mut a = audio();
    eval!(&mut a,
        < EventAudio::MenuOpened,
        > ActionAudio::SampleOutputs,
        > ActionAudio::Sample,
    );
}

/// The list is the expensive half of this module and nothing outside the panel
/// shows it, so a timer tick with the panel shut asks only for the level.
#[test]
fn a_shut_panel_never_lists_the_outputs() {
    let mut a = audio();
    eval!(&mut a,
        < EventAudio::Due,
        > ActionAudio::Sample,
    );

    eval!(&mut a,
        < EventAudio::MenuOpened,
        > ActionAudio::SampleOutputs,
        > ActionAudio::Sample,

        < EventAudio::Due,
        > ActionAudio::SampleOutputs,
        > ActionAudio::Sample,

        < EventAudio::MenuClosed,
        < EventAudio::Due,
        > ActionAudio::Sample,
    );
}

#[test]
fn picking_an_output_switches_to_it() {
    let mut a = audio();
    eval!(&mut a,
        < seen("speaker"),

        < EventAudio::OutputPicked { id: "phones".into() },
        > ActionAudio::SetDefaultOutput { .. },
    );
    assert!(
        var(&a, "vol_outputs")
            .contains(r#""id":"phones","id_b64":"cGhvbmVz","name":"AirPods Pro","active":true"#),
        "the tick moves at once rather than waiting for the listing: {}",
        var(&a, "vol_outputs")
    );
}

/// The same trap the slider has: a listing that was already running describes
/// the sink from before the click, and believing it moves the tick back under
/// the finger that just clicked.
#[test]
fn a_listing_from_before_the_switch_does_not_undo_it() {
    let mut a = audio();
    eval!(&mut a,
        < seen("speaker"),

        < EventAudio::OutputPicked { id: "phones".into() },
        > ActionAudio::SetDefaultOutput { .. },

        < seen("speaker"),
    );
    assert!(
        var(&a, "vol_outputs").contains(r#""name":"AirPods Pro","active":true"#),
        "the stale listing was ignored"
    );

    eval!(&mut a,
        < EventAudio::OutputSwitched { ok: true },
        > ActionAudio::SampleOutputs,
        > ActionAudio::Sample,
    );
}

/// Volume is a property of the sink, so the slider is describing the old device
/// until a fresh level arrives.
#[test]
fn switching_output_re_reads_the_level() {
    let mut a = audio();
    eval!(&mut a,
        < seen("speaker"),
        < EventAudio::OutputPicked { id: "phones".into() },
        > ActionAudio::SetDefaultOutput { .. },

        < EventAudio::OutputSwitched { ok: true },
        > ActionAudio::SampleOutputs,
        > ActionAudio::Sample,
    );
}

#[test]
fn clicking_the_output_already_in_use_is_not_a_command() {
    let mut a = audio();
    eval!(&mut a,
        < seen("speaker"),

        < EventAudio::OutputPicked { id: "speaker".into() },
    );
}

/// Two rows mashed together are one situation. The first is acted on and the
/// second is a mis-click, exactly as a second Wi-Fi row is while a join runs.
#[test]
fn a_second_pick_while_one_is_in_flight_is_ignored() {
    let mut a = audio();
    eval!(&mut a,
        < seen("speaker"),

        < EventAudio::OutputPicked { id: "phones".into() },
        < EventAudio::OutputPicked { id: "speaker".into() },
        > ActionAudio::SetDefaultOutput { .. },
    );
    assert!(
        var(&a, "vol_outputs").contains(r#""name":"AirPods Pro","active":true"#),
        "the first click is the one that counted"
    );
}

#[test]
fn the_active_row_gets_the_glyph_that_reads_on_the_accent_circle() {
    let mut a = audio();
    a.set_icons_dir("/icons/light");
    a.handle_events(&[seen("phones")]);
    let outputs = var(&a, "vol_outputs");
    assert!(
        outputs.contains("/icons/light/audio-out-headphones-on-symbolic.svg"),
        "the sink in use: {outputs}"
    );
    assert!(
        outputs.contains("/icons/light/audio-out-speaker-symbolic.svg"),
        "and the one that is not: {outputs}"
    );
}

#[test]
fn no_outputs_is_an_empty_list_rather_than_a_missing_var() {
    let a = audio();
    assert_eq!(var(&a, "vol_outputs"), "[]");
}
