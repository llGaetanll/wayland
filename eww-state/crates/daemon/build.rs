//! Bake the commit into the binary, so every log line says which build wrote it.

use std::process::Command;

fn main() {
    // A tree with no git still builds; it logs `unknown` rather than nothing.
    let sha = Command::new("git")
        .args(["rev-parse", "--short=6", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=EWW_STATE_COMMIT={sha}");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
}
