//! The floor every module stands on.
//!
//! Deliberately small, and deliberately ignorant: nothing here knows that this
//! program has a Wi-Fi menu or a dock. It is the handful of types a module and
//! its parent both have to name — an id, a clock reading, the shape of the eww
//! variables that go out — and nothing that would make adding a widget a reason
//! to edit a shared file.
//!
//! No IO of any kind, and no dependencies.

/// Assert that a wire value serializes to exactly this text.
///
/// The expected string is the contract, so it is written out rather than
/// snapshotted. See the wire-type rules in docs/STYLE.md.
#[cfg(test)]
macro_rules! serializes {
    ($value:expr, $expected:expr) => {
        assert_eq!($value.to_string(), $expected);
    };
}

pub mod jobs;
pub mod millis;
pub mod module;
pub mod util;
pub mod wire_json;
pub mod wire_vars;

pub use jobs::JobId;
pub use jobs::Jobs;
pub use millis::Millis;
pub use module::Module;
pub use wire_json::WireJson;
pub use wire_vars::WireVars;
