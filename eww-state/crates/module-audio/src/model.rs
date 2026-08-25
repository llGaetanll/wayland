//! What the world looks like, as far as volume is concerned.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Audio {
    pub volume: u8,
    pub muted: bool,
}
