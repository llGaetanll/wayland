//! An installed application, as the launcher sees it.

/// One `.desktop` entry, reduced to what a launcher needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    /// What the user reads, from `Name=`.
    pub name: String,
    /// The command to run, with the `%f`-style field codes already stripped.
    pub exec: String,
    /// Absolute path to an icon file eww can draw, or empty when none was
    /// found. Always a PNG: this machine has no gdk-pixbuf SVG loader.
    pub icon: String,
    /// `GenericName=` and `Keywords=` folded into one haystack. Matched but
    /// never shown, so that typing "browser" finds Firefox.
    pub keywords: String,
}
