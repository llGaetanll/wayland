//! Side effects the machine asks the shell to perform.
//!
//! An `Action` is a request, not a result: nothing here runs, and the machine
//! never waits on one. Every action carries a [`JobId`]; when the outcome
//! matters, the shell reports it back as `EventJob::Finished` carrying the same
//! id, so a result can never be attributed to the wrong request.

use crate::action_file::ActionFile;
use crate::action_probe::ActionProbe;
use crate::action_shell::ActionShell;
use crate::probe::JobId;

#[derive(Debug)]
pub enum Action {
    /// Sample the world.
    ToProbe { id: JobId, action: ActionProbe },
    /// Run a program.
    ToShell { id: JobId, action: ActionShell },
    /// Touch a file we own.
    ToFile { id: JobId, action: ActionFile },
}

impl Action {
    pub fn probe(id: JobId, what: crate::probe::Probe) -> Action {
        Action::ToProbe {
            id,
            action: ActionProbe::Sample { what },
        }
    }

    pub fn shell(id: JobId, action: ActionShell) -> Action {
        Action::ToShell { id, action }
    }

    pub fn file(id: JobId, action: ActionFile) -> Action {
        Action::ToFile { id, action }
    }

    pub fn notify(id: JobId, title: &str, body: impl Into<String>) -> Action {
        Action::shell(
            id,
            ActionShell::Notify {
                title: title.to_string(),
                body: body.into(),
            },
        )
    }

    pub fn id(&self) -> JobId {
        match self {
            Action::ToProbe { id, .. } | Action::ToShell { id, .. } | Action::ToFile { id, .. } => {
                *id
            }
        }
    }
}
