//! The eww variable table.
//!
//! Derived from state on the way out, never stored beside it. The runtime diffs
//! this against what it last published, so a machine can republish freely and
//! only real changes reach eww.

use std::collections::BTreeMap;
use std::fmt::Display;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WireVars(BTreeMap<&'static str, String>);

impl WireVars {
    pub fn set(&mut self, name: &'static str, value: impl Display) {
        self.0.insert(name, value.to_string());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|s| s.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> + '_ {
        self.0.iter().map(|(k, v)| (*k, v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
