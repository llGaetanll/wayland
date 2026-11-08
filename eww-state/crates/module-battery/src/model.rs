//! What the world looks like, as far as the battery is concerned.

/// What the bar's pill draws.
///
/// Deliberately only these two. The pill's cache path is keyed on this value,
/// so a field that moves every sample — watts, seconds left — would have the
/// glyph redrawn every fifteen seconds for a picture that did not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

/// Everything the panel says about the pack. Read whole from one pass over
/// `/sys/class/power_supply/BAT0`, so the parts cannot disagree with each other
/// the way two separately-timed reads can.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Reading {
    pub battery: Battery,
    /// Present draw in milliwatts, `voltage_now * current_now`. Zero when the
    /// kernel reports no current, which is what a full pack on the cable does.
    pub milliwatts: u32,
    /// Seconds until empty, or until full while charging. `None` when the
    /// current is zero and the division has no answer — the panel says so
    /// rather than showing a number that is really a division by nothing.
    pub seconds_left: Option<u32>,
    /// `POWER_SUPPLY_HEALTH`, as the kernel spells it.
    pub health: String,
    pub cycles: u32,
}

/// One point on the chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistPoint {
    /// Unix seconds, not the daemon's own clock. The ring outlives the process
    /// — it is on disk precisely so a restart does not blank the last twelve
    /// hours — and a monotonic reading means nothing across a reboot.
    pub at: u64,
    pub percent: u8,
    pub charging: bool,
}

/// One row of the energy list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proc {
    pub name: String,
    /// Share of the machine's *busy* CPU time over the sampling window, as a
    /// whole percent — not of its capacity. Idle is excluded on purpose: the
    /// question the panel answers is which app is spending the energy being
    /// spent, and against capacity every row on a 12-thread machine reads as a
    /// single digit whatever it is doing.
    pub share: u8,
    /// How long it has been up this boot, in seconds.
    pub uptime: u64,
    pub icon: String,
}
