//! The open pull requests of the repository on GitHub, by the name of their branch (`github.read-pull-requests`).

use std::collections::BTreeMap;
use std::fmt;

/// What the checks on the last commit of a pull request say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checks {
    /// They all passed: `SUCCESS`.
    Passed,
    /// One failed: `FAILURE` or `ERROR`.
    Failed,
    /// Some still run: `PENDING` or `EXPECTED`.
    Running,
}

/// An open pull request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    /// Its number: 12 for #12.
    pub number: u64,
    /// Its title.
    pub title: String,
    /// Its address, as GitHub gave it.
    pub url: String,
    /// Whether it is a draft.
    pub is_draft: bool,
    /// The name of its branch on GitHub: `feature/login`.
    pub branch: String,
    /// What the checks of its last commit say; none when it has none, or they say something else.
    pub checks: Option<Checks>,
}

/// The open pull requests of the repository, one per branch name: the one updated last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullRequests {
    /// The pull requests, by the name of their branch on GitHub.
    pub by_branch: BTreeMap<String, PullRequest>,
}

impl PullRequests {
    /// The open pull request of a branch name on GitHub, when one is known.
    #[must_use]
    pub fn of_branch(&self, branch: &str) -> Option<&PullRequest> {
        self.by_branch.get(branch)
    }
}

/// Whether git's credential helper may be asked for the GitHub token (`github.token`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperAsked {
    /// It may: only **Load pull requests from GitHub** asks it.
    Allowed,
    /// It may not: the reads made on their own use only the token kept.
    NotAllowed,
}

/// A GitHub token. It is never shown: not in `Debug`, not in a message, not in the log.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(pub String);

impl fmt::Debug for Token {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Token(<hidden>)")
    }
}
