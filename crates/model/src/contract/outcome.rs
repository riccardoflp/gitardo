//! What a feature says it needs next, or what it has decided to do.

use super::{AnswerKey, Answers, Context, Plan, Question, Text};
use crate::CommitId;

/// The identifier of a feature, as its card is headed: `branch.create`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureId(pub &'static str);

/// An operation of the catalog: called again and again with what is known so far, it says what it needs next. It is
/// pure (rule R2.2): the same context and answers always give the same outcome.
pub type Operation = fn(&Context, &Answers) -> Outcome;

/// What an operation needs next, or what it has decided to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Ask the user something; the answer comes back under the question's key.
    Ask(Question),
    /// Read something more from the repository; the result comes back under the key given.
    Read(AnswerKey, Query),
    /// Do not run: why, in words for the user.
    Refuse(Refusal),
    /// Run this.
    Run(Plan),
}

/// Something a feature needs read from the repository before it can decide. The engine reads it and gives the result
/// back as an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// The last entries of the reflog of a reference, newest first: `HEAD` or `refs/heads/main`. The answer is
    /// [`super::Answer::Reflog`].
    Reflog {
        /// The reference, in full, or `HEAD`.
        reference: String,
        /// How many entries, at most.
        count: usize,
    },
    /// Whether the files of the working tree are as a commit has them (`git diff --quiet <commit>`). The answer is
    /// [`super::Answer::Flag`].
    FilesMatch {
        /// The commit.
        commit: CommitId,
    },
    /// The newest entry of the engine's journal that still holds for this repository. The answer is
    /// [`super::Answer::Journal`].
    Journal,
}

/// Why a feature does not run, in words for the user, with what may be done instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The sentence: "This repository has no remote."
    pub message: Text,
    /// A feature offered in its place, with the words of its button: **Add remote…**.
    pub offer: Option<(FeatureId, Text)>,
}

impl Refusal {
    /// A refusal that offers nothing else.
    #[must_use]
    pub fn saying(message: &str) -> Self {
        Refusal {
            message: Text::from(message),
            offer: None,
        }
    }
}
