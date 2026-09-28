//! Small pure helpers.

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64, padded.
///
/// SSIDs and device names reach a widget handler as part of a shell command, so
/// they travel base64-encoded: no quoting rules to get wrong, and nothing a
/// hostile access point can name itself gets to run.
pub fn b64(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Whether a short app id and a window class name the same app.
///
/// The two come from different registries and rarely agree letter for letter:
/// `alacritty` vs `Alacritty`, `signal` vs `signal-desktop`. Case-folded
/// containment either way is enough to tell apps apart from each other, and is
/// wrong only for ids that are substrings of unrelated classes — which is why
/// the id is the user's own short name and not a fragment.
pub fn class_matches(id: &str, class: &str) -> bool {
    let id = id.to_ascii_lowercase();
    let class = class.to_ascii_lowercase();
    id == class || class.contains(&id) || id.contains(&class)
}

#[cfg(test)]
mod tests {
    use super::b64;
    use super::class_matches;

    #[test]
    fn matches_reference_vectors() {
        assert_eq!(b64(""), "");
        assert_eq!(b64("f"), "Zg==");
        assert_eq!(b64("fo"), "Zm8=");
        assert_eq!(b64("foo"), "Zm9v");
        assert_eq!(b64("foobar"), "Zm9vYmFy");
        assert_eq!(b64("Café ✳"), "Q2Fmw6kg4pyz");
    }

    #[test]
    fn an_id_and_a_class_can_name_the_same_app() {
        assert!(class_matches("alacritty", "Alacritty"));
        assert!(class_matches("signal", "signal-desktop"));
        assert!(class_matches("firefox", "firefox"));
        assert!(!class_matches("discord", "firefox"));
    }
}
