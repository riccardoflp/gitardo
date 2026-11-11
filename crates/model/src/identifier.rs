//! The names of things git knows by a number.

use std::fmt;

/// The full hash of a commit, or of any object git names: 40 hexadecimal digits, or 64 in a repository of SHA-256.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitId(String);

impl CommitId {
    /// The hash, when the text is one: 40 or 64 hexadecimal digits in lower case, as git prints them.
    #[must_use]
    pub fn parse(text: &str) -> Option<CommitId> {
        let is_hexadecimal = text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        (is_hexadecimal && (text.len() == 40 || text.len() == 64)).then(|| CommitId(text.to_owned()))
    }

    /// The hash in full.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The first seven digits, as the graph and the messages show a commit.
    #[must_use]
    pub fn short(&self) -> &str {
        self.0.get(..7).unwrap_or(&self.0)
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A moment, in seconds since 1970-01-01 00:00:00 UTC, as git writes dates with `%at` and `:unix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Timestamp(pub i64);

#[cfg(test)]
mod tests {
    use super::CommitId;

    #[test]
    fn takes_only_full_hashes_as_git_prints_them() {
        let sha1 = "3f2a1c9d0e4b5a6978877665544332211aabbccd";
        let cases = [
            (sha1.to_owned(), true),
            (String::from("3f2a1c9"), false),
            (sha1.to_uppercase(), false),
            ("g".repeat(40), false),
            ("a".repeat(64), true),
        ];
        for (text, expected) in cases {
            assert_eq!(CommitId::parse(&text).is_some(), expected, "{text}");
        }
        assert_eq!(CommitId::parse(sha1).unwrap().short(), "3f2a1c9");
    }
}
