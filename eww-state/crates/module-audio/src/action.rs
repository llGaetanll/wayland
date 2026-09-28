//! What the volume module can ask the outside world to do.
//!
//! `Set` is waited on but needs no id: the module counts how many are out
//! rather than telling them apart, because the only question it asks is
//! *have they all finished*.

#[derive(Debug)]
pub enum ActionAudio {
    Sample,
    Set {
        percent: u8,
    },
    SampleOutputs,
    /// Make this sink the one new streams go to. At most one is ever out, so
    /// like `Set` it needs no id to be recognised on the way back.
    SetDefaultOutput {
        id: String,
    },
}
