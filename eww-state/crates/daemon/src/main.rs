//! `eww-state` — the daemon that owns the desktop widgets' state, and the CLI
//! the widgets themselves call.
//!
//! Almost nothing lives here. Each widget is its own `module-*` crate holding
//! both its state machine and the commands that serve it; `module-bar` routes
//! between them. What is left in this crate is the loop, the control socket,
//! the timers, and making eww match what the machine says should be true.

mod eww_cli;
mod hypr;
mod ipc;
mod log;
mod paths;
mod reconcile;
mod runtime_daemon;
mod runtime_perform;
mod ticker;

use paths::Paths;
use runtime_daemon::RuntimeDaemon;

const USAGE: &str = "\
eww-state — widget state daemon

  eww-state daemon              run the daemon (start this from the compositor)

  eww-state menu toggle <wifi|bt|volume|brightness>
  eww-state menu close
  eww-state volume set <0-100>
  eww-state brightness set <1-100>
  eww-state wifi toggle | wifi connect <ssid-base64> | wifi rescan
  eww-state bt toggle | bt connect <mac-base64>
  eww-state dock launch <id> | dock swap <a> <b>
  eww-state shot capture | shot <copy|save|discard|close>
  eww-state window close        close the focused window, leaving its Space first
  eww-state window minimize     put the focused window away in the dock
  eww-state window restore <address>
  eww-state refresh <probe>

  eww-state dump | vars | windows | polls | ping
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths = Paths::discover();

    match args.first().map(String::as_str) {
        // Help and query answers are the program's output rather than a record
        // of it, so they go to stdout; everything else goes through tracing.
        None | Some("-h") | Some("--help") | Some("help") => print!("{USAGE}"),
        Some("daemon") => {
            log::init_daemon(paths.log.clone());
            if let Err(e) = RuntimeDaemon::run(paths) {
                tracing::error!(error = %e, "the daemon stopped");
                std::process::exit(1);
            }
        }
        _ => {
            log::init_client();
            client(&paths, &args);
        }
    }
}

fn client(paths: &Paths, args: &[String]) {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    // Check the command here as well as in the daemon, so a typo fails loudly
    // at the caller instead of being swallowed by a widget handler.
    if let Err(e) = ipc::parse(&refs) {
        tracing::error!("{e}");
        std::process::exit(2);
    }
    match ipc::request(&paths.socket, args) {
        Ok(reply) => {
            let reply = reply.trim_end();
            if let Some(err) = reply.strip_prefix("error: ") {
                tracing::error!("{err}");
                std::process::exit(1);
            }
            if !reply.is_empty() && reply != "ok" {
                println!("{reply}");
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "daemon not reachable");
            std::process::exit(1);
        }
    }
}

/// The shared accent colour, without its leading `#`, for theming slurp.
///
/// Read from `~/.config/colors.sh`, the same file the stylesheets mirror, so
/// there is one place to change it.
pub fn accent_hex() -> String {
    let fallback = "0a84ff".to_string();
    let Some(home) = std::env::var_os("HOME") else {
        return fallback;
    };
    let path = std::path::PathBuf::from(home).join(".config/colors.sh");
    let Ok(text) = std::fs::read_to_string(path) else {
        return fallback;
    };
    text.lines()
        .find_map(|l| l.trim().strip_prefix("ACCENT="))
        .map(|v| {
            v.trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .trim_start_matches('#')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback)
}
