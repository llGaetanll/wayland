//! What the world looks like, as far as the battery is concerned.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}
