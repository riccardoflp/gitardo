//! Words the user reads.

use std::fmt;

/// A sentence or a label the user reads, word for word as its card says it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Text(String);

impl Text {
    /// The words.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Text {
    fn from(words: &str) -> Self {
        Text(words.to_owned())
    }
}

impl From<String> for Text {
    fn from(words: String) -> Self {
        Text(words)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
