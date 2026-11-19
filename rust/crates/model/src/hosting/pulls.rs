//! The open pull requests of the main remote to pick one from, and the number of one typed
//! (`hosting.read-pull-request-list`, `hosting.checkout-pull-request`).

/// An open pull request, from the repository itself or from a fork, as the pick of **Check out a pull request…**
/// shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullListEntry {
    /// Its number: 12.
    pub number: u64,
    /// Its title.
    pub title: String,
    /// Who opened it: `ada`.
    pub author: String,
    /// Its branch, in the repository it comes from: `fix-parser`.
    pub branch: String,
    /// The repository it comes from, `contributor/app`, when it is not the repository of the main remote; none when it
    /// is, or when the service does not say.
    pub fork: Option<String>,
}

impl PullListEntry {
    /// The owner of the fork it comes from: `contributor` of `contributor/app`; none when it is not from a fork.
    #[must_use]
    pub fn fork_owner(&self) -> Option<&str> {
        let fork = self.fork.as_deref()?;
        Some(fork.split('/').next().unwrap_or(fork))
    }
}

/// The open pull requests read, or why they were not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullList {
    /// Those listed, newest first; none on a service whose pull requests are not read.
    Listed(Vec<PullListEntry>),
    /// They could not be read: why, in words.
    Unread(String),
}

/// The number of a pull request typed: `12`, `#12` or `!12`, spaces around left out; none for anything else.
#[must_use]
pub fn typed_number(typed: &str) -> Option<u64> {
    let typed = typed.trim();
    let digits = typed.strip_prefix(['#', '!']).unwrap_or(typed);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|number| *number > 0)
}
