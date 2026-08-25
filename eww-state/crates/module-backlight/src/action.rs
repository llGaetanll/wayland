//! What the brightness module can ask the outside world to do.

#[derive(Debug)]
pub enum ActionBacklight {
    Sample,
    Set { percent: u8 },
}
