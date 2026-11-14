//! What can go wrong when the engine asks something of the operating system.

use std::fmt;

/// Why the operating system did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemError {
    /// The system refused, and said why in its own words.
    System {
        /// What was asked: the call, or what it was asked for.
        call: &'static str,
        /// What the system said, kept to be shown under the details of the error.
        details: String,
    },
    /// The system cannot do what was asked, such as keeping secrets where Gitardo has no store to speak to.
    Unsupported {
        /// What was asked.
        what: &'static str,
    },
}

impl SystemError {
    /// The error of a call that the system refused, with its words.
    pub(crate) fn system(call: &'static str, error: &impl fmt::Display) -> SystemError {
        SystemError::System {
            call,
            details: error.to_string(),
        }
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemError::System { call, details } => write!(formatter, "The system refused {call}: {details}"),
            SystemError::Unsupported { what } => write!(formatter, "This system cannot {what}."),
        }
    }
}

impl std::error::Error for SystemError {}
