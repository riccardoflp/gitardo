//! What can go wrong when Gitardo runs git or reads what it printed.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why git could not answer, in words the user can act on, with what git printed kept for **Details**.
#[derive(Debug)]
pub enum GitError {
    /// No `git` could be started: it is not installed, or not on the `PATH`.
    NotFound {
        /// What the system said.
        source: io::Error,
    },
    /// Git ran and failed.
    Failed {
        /// The command, as the log writes it.
        command: String,
        /// Its exit code, none when it was ended by a signal.
        code: Option<i32>,
        /// What it printed on its standard error.
        stderr: String,
    },
    /// The command was cancelled before it ended, and its process killed.
    Cancelled {
        /// The command, as the log writes it.
        command: String,
    },
    /// Talking to the process failed: its output could not be read, or its input written.
    Process {
        /// The command, as the log writes it.
        command: String,
        /// What the system said.
        source: io::Error,
    },
    /// Git printed something this program does not understand.
    Unexpected {
        /// What was being read: `the branches`.
        what: &'static str,
        /// The part that could not be read.
        detail: String,
    },
    /// A read was not run, because what it was asked cannot be read: two sides of a comparison that are not commits,
    /// a name that is not a stash.
    Refused {
        /// Why, in the words of the card: *These are not commits of this repository.*
        reason: String,
    },
    /// A file of the working tree could not be read.
    File {
        /// The file.
        path: PathBuf,
        /// What the system said.
        source: io::Error,
    },
}

impl GitError {
    /// What git printed on its standard error, when it ran and failed.
    #[must_use]
    pub fn stderr(&self) -> Option<&str> {
        match self {
            GitError::Failed { stderr, .. } => Some(stderr),
            GitError::NotFound { .. }
            | GitError::Cancelled { .. }
            | GitError::Process { .. }
            | GitError::Unexpected { .. }
            | GitError::Refused { .. }
            | GitError::File { .. } => None,
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotFound { source } => {
                write!(
                    formatter,
                    "Git could not be started: install Git and make sure it is on the PATH ({source})."
                )
            }
            GitError::Failed { command, stderr, .. } => {
                let said = stderr.trim();
                if said.is_empty() {
                    write!(formatter, "`{command}` failed.")
                } else {
                    write!(formatter, "`{command}` failed: {said}")
                }
            }
            GitError::Cancelled { command } => write!(formatter, "`{command}` was cancelled."),
            GitError::Process { command, source } => {
                write!(formatter, "`{command}` could not be run to its end: {source}")
            }
            GitError::Unexpected { what, detail } => {
                write!(
                    formatter,
                    "Git said something unexpected while reading {what}: {detail}"
                )
            }
            GitError::Refused { reason } => formatter.write_str(reason),
            GitError::File { path, source } => {
                write!(formatter, "{} could not be read: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for GitError {}
