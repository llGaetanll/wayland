//! Running other programs, which is most of what this crate does.

use std::io;
use std::process::Command;
use std::process::Stdio;

/// Run a command and return its stdout, trimmed. `None` if it could not be run
/// or exited non-zero — every caller treats those the same way.
pub fn out(program: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).trim_end().to_string())
}

/// Run a command for its effect. Returns whether it succeeded.
pub fn ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Feed `input` to a command's stdin, for putting bytes on the clipboard.
pub fn pipe_file(program: &str, args: &[&str], path: &str) -> io::Result<bool> {
    let file = std::fs::File::open(path)?;
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::from(file))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(status.success())
}

/// Launch a GUI app and forget about it: new session, no inherited stdio, and
/// no zombie left behind when it exits long after we stopped caring.
pub fn detach(cmd: &str) {
    let _ = Command::new("setsid")
        .args(["-f", "sh", "-c", cmd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
