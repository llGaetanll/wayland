//! Minimal JSON writer.
//!
//! eww vars that widgets index into (`wifi.networks`, `dock_apps`) have to be
//! JSON text. This is the whole of what we need to produce it; nothing here
//! parses, so there is no error type.

use std::fmt;
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq)]
pub enum WireJson {
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<WireJson>),
    Obj(Vec<(&'static str, WireJson)>),
}

impl WireJson {
    pub fn s(v: impl Into<String>) -> WireJson {
        WireJson::Str(v.into())
    }
}

impl fmt::Display for WireJson {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireJson::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            WireJson::Int(i) => write!(f, "{i}"),
            WireJson::Str(s) => write_str(f, s),
            WireJson::Arr(items) => {
                f.write_char('[')?;
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write!(f, "{it}")?;
                }
                f.write_char(']')
            }
            WireJson::Obj(fields) => {
                f.write_char('{')?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_char(',')?;
                    }
                    write_str(f, k)?;
                    f.write_char(':')?;
                    write!(f, "{v}")?;
                }
                f.write_char('}')
            }
        }
    }
}

fn write_str(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_nests() {
        serializes!(
            WireJson::Obj(vec![
                ("ssid", WireJson::s("a\"b\\c")),
                ("on", WireJson::Bool(true)),
                (
                    "list",
                    WireJson::Arr(vec![WireJson::Int(1), WireJson::Int(-2)]),
                ),
            ]),
            r#"{"ssid":"a\"b\\c","on":true,"list":[1,-2]}"#
        );
    }

    #[test]
    fn escapes_control_chars() {
        serializes!(WireJson::s("a\u{1}b"), r#""a\u0001b""#);
    }
}
