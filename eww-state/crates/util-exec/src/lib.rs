//! Running other programs.
//!
//! Every module that talks to the outside world does it through here, so what
//! happens to a command's output is decided once: trimmed, and a failure is
//! indistinguishable from no output, because every caller treats them alike.
//!
//! What a command says on stderr is logged rather than discarded. Throwing it
//! away costs nothing until something goes wrong, and then it is the one line
//! that would have said why.

use std::io;
use std::io::Read;
use std::process::Command;
use std::process::Stdio;

/// Run a command and return its stdout, trimmed. `None` if it could not be run
/// or exited non-zero — every caller treats those the same way.
pub fn out(program: &str, args: &[&str]) -> Option<String> {
    let o = match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(program, ?args, error = %e, "could not run");
            return None;
        }
    };
    log_stderr(program, args, &o.stderr);
    if !o.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&o.stdout).trim_end().to_string())
}

/// Run a command for its effect. Returns whether it succeeded.
pub fn ok(program: &str, args: &[&str]) -> bool {
    let spawned = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(program, ?args, error = %e, "could not run");
            return false;
        }
    };
    // Reading stderr here would wait for every process holding it to let go,
    // and an eww client that cannot reach a server forks itself into one that
    // keeps the stderr it was given. That fork outlives the client by the rest
    // of the session, so the pipe is drained on a thread of its own and only
    // the client itself is waited for.
    if let Some(stderr) = child.stderr.take() {
        drain(program, args, stderr);
    }
    match child.wait() {
        Ok(status) => status.success(),
        Err(e) => {
            tracing::warn!(program, ?args, error = %e, "could not wait for");
            false
        }
    }
}

/// Log what a command says on stderr, on a thread, whenever it finishes saying
/// it. Nothing waits for this.
fn drain(program: &str, args: &[&str], mut stderr: std::process::ChildStderr) {
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let spawned = std::thread::Builder::new()
        .name("stderr".into())
        .spawn(move || {
            let mut said = Vec::new();
            if stderr.read_to_end(&mut said).is_err() {
                return;
            }
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            log_stderr(&program, &args, &said);
        });
    if let Err(e) = spawned {
        tracing::warn!(error = %e, "could not read a command's stderr");
    }
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

/// Say what a command said on stderr, whether or not it went on to succeed.
///
/// Exit status is not the whole story: an eww client that cannot reach a server
/// starts one, says so here, and still exits zero.
fn log_stderr(program: &str, args: &[&str], stderr: &[u8]) {
    let said = String::from_utf8_lossy(stderr);
    let said = said.trim();
    if !said.is_empty() {
        tracing::warn!(program, ?args, stderr = said, "command wrote to stderr");
    }
}
