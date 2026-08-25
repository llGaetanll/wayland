//! Milliseconds since an epoch the runtime chose.
//!
//! A plain number, because the tick carrying it has to survive being written to
//! a trace and read back in another process — see [REPLAY.md]. `Instant` cannot:
//! it is opaque, monotonic from an arbitrary origin, and meaningless in another
//! run. It is also a foreign type in a payload, which rule 9 objects to on its
//! own.
//!
//! [REPLAY.md]: https://github.com/eww-state/docs/REPLAY.md

/// Milliseconds since the runtime's epoch. Zero is "no tick has arrived yet",
/// which is why the default is useful rather than a lie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Millis(pub u64);

impl Millis {
    /// How long since `earlier`, saturating at zero rather than panicking if
    /// the two ever arrive out of order.
    pub fn since(self, earlier: Millis) -> Millis {
        Millis(self.0.saturating_sub(earlier.0))
    }

    pub fn secs(n: u64) -> Millis {
        Millis(n.saturating_mul(1_000))
    }
}

impl std::ops::Add for Millis {
    type Output = Millis;

    fn add(self, rhs: Millis) -> Millis {
        Millis(self.0.saturating_add(rhs.0))
    }
}

impl std::fmt::Display for Millis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::Millis;

    #[test]
    fn elapsed_never_goes_backwards() {
        assert_eq!(Millis(500).since(Millis(200)), Millis(300));
        assert_eq!(
            Millis(200).since(Millis(500)),
            Millis(0),
            "a clock that stepped back reads as no time passed"
        );
    }
}
