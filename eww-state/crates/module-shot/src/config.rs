//! What this module has to be told rather than work out.

use api::Millis;

#[derive(Clone, Debug)]
pub struct ConfigShot {
    /// Widest the preview may be drawn.
    pub max_width: u32,
    /// How long a region selection may be up before the module stops believing
    /// in it. Patience rather than a stuck job: the user is holding the mouse.
    pub timeout: Millis,
}

impl Default for ConfigShot {
    fn default() -> Self {
        ConfigShot {
            max_width: 320,
            timeout: Millis::secs(180),
        }
    }
}
