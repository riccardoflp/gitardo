//! What a check found wrong, said so that it can be fixed.

use std::fmt;

/// Something in the workspace that breaks a rule of `docs/03-code-rules.md`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Problem {
    /// The number of the rule it breaks: `R1.1`.
    pub rule: &'static str,
    /// Where: a file, with a line when there is one.
    pub place: String,
    /// What is wrong and what to do, in a sentence.
    pub message: String,
}

impl Problem {
    /// A problem in a file, at a line counted from 1.
    pub fn at_line(rule: &'static str, file: &str, line: usize, message: String) -> Self {
        Problem {
            rule,
            place: format!("{file}:{line}"),
            message,
        }
    }

    /// A problem about a whole file or crate.
    pub fn in_file(rule: &'static str, file: &str, message: String) -> Self {
        Problem {
            rule,
            place: file.to_owned(),
            message,
        }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}: {}", self.rule, self.place, self.message)
    }
}
