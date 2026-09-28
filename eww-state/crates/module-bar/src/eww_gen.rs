//! Which fill of eww's variable table we are looking at.

/// A generation of the published variable table.
///
/// eww survives a `reload` no better than a restart as far as its variables are
/// concerned — the config is read again and every `defvar` is back at the
/// default written in it — but the socket is the same inode either way, so
/// [`crate::ServerId`] cannot see it. The machine stamps this number into a var
/// of eww's own and reads it back: a table carrying the current generation is
/// one we filled, and anything else is one we have to fill again.
///
/// The value means nothing beyond being different from the last one. It counts
/// rather than being random so that reads can be ordered: one taken before a
/// stamp landed reports an older generation, and older is news from a world
/// that has already been replaced rather than a second reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct EwwGen(u64);

impl EwwGen {
    /// The generation after this one. Never [`EwwGen::default`], so the value
    /// an unstamped table reads back as is one we never publish ourselves.
    pub fn next(self) -> EwwGen {
        EwwGen(self.0.saturating_add(1))
    }

    /// What a read of the sentinel var says, if it says anything. The empty
    /// string is the `defvar` default and means unstamped; so does anything
    /// that will not parse, which is a var somebody else has been writing.
    pub fn parse(text: &str) -> Option<EwwGen> {
        match text.trim().parse::<u64>() {
            Ok(0) | Err(_) => None,
            Ok(n) => Some(EwwGen(n)),
        }
    }
}

impl std::fmt::Display for EwwGen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::EwwGen;

    #[test]
    fn an_unstamped_table_reads_back_as_nothing() {
        assert_eq!(EwwGen::parse(""), None);
        assert_eq!(EwwGen::parse("  "), None);
        assert_eq!(EwwGen::parse("light"), None);
        // Never published, so reading it back is the same as reading nothing.
        assert_eq!(EwwGen::parse("0"), None);
    }

    #[test]
    fn a_stamp_survives_the_round_trip() {
        let gen = EwwGen::default().next().next();
        assert_eq!(EwwGen::parse(&gen.to_string()), Some(gen));
        assert!(gen > EwwGen::default().next());
    }
}
