//! What can stop a chore of the project before it has an answer.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Why a chore could not run to its end.
#[derive(Debug)]
pub enum Error {
    /// A file or folder of the workspace could not be read.
    Read {
        /// The file or folder.
        path: PathBuf,
        /// What the system said.
        source: io::Error,
    },
    /// A manifest or lock file is not TOML this reader understands.
    Toml {
        /// The file.
        path: PathBuf,
        /// The line, from 1.
        line: usize,
        /// What is wrong there.
        message: String,
    },
    /// The report could not be written to the terminal.
    Output(io::Error),
    /// A tool of the toolchain could not be started.
    Tool {
        /// The command, as it would be typed.
        command: String,
        /// What the system said.
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => write!(formatter, "Could not read {}: {source}", path.display()),
            Error::Toml { path, line, message } => write!(formatter, "{}:{line}: {message}", path.display()),
            Error::Output(source) => write!(formatter, "Could not write the report: {source}"),
            Error::Tool { command, source } => write!(formatter, "Could not start `{command}`: {source}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Error::Output(source)
    }
}
