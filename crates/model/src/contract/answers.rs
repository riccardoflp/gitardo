//! What a feature has learned so far: the answers of its questions and the results of its queries.

use std::collections::BTreeMap;

use crate::CommitId;

use super::JournalEntry;

/// The name under which an answer or the result of a query comes back to the feature that asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnswerKey(pub &'static str);

/// One answer, or one result of a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The text typed in an input box.
    Text(String),
    /// The choice picked, by its identifier.
    Choice(String),
    /// The choices ticked, by their identifiers.
    Choices(Vec<String>),
    /// A confirmation accepted.
    Confirmed,
    /// The entries of a reflog, newest first.
    Reflog(Vec<ReflogEntry>),
    /// The answer of a query that says yes or no.
    Flag(bool),
    /// The commit a name stands for, or none when git knows no commit by it.
    Commit(Option<CommitId>),
    /// The newest entry of the journal that still holds, if any.
    Journal(Option<JournalEntry>),
}

/// An entry of a reflog: where a reference was, and what moved it there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflogEntry {
    /// The commit the reference was at after the move.
    pub commit: CommitId,
    /// What moved it: `commit: Fix the parser`, `checkout: moving from main to feature`.
    pub subject: String,
}

/// The answers and results a feature has so far, by key. It starts empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answers(BTreeMap<AnswerKey, Answer>);

impl Answers {
    /// No answers yet.
    #[must_use]
    pub fn new() -> Self {
        Answers::default()
    }

    /// The answers with one more; an answer under the same key is replaced.
    #[must_use]
    pub fn with(mut self, key: AnswerKey, answer: Answer) -> Self {
        self.insert(key, answer);
        self
    }

    /// Takes one more answer; an answer under the same key is replaced.
    pub fn insert(&mut self, key: AnswerKey, answer: Answer) {
        let _replaced = self.0.insert(key, answer);
    }

    /// The answer under a key.
    #[must_use]
    pub fn get(&self, key: AnswerKey) -> Option<&Answer> {
        self.0.get(&key)
    }

    /// The text typed for a key, when the answer is text.
    #[must_use]
    pub fn text(&self, key: AnswerKey) -> Option<&str> {
        match self.get(key)? {
            Answer::Text(text) => Some(text),
            _ => None, // Any other kind of answer is no text.
        }
    }

    /// The choice picked for a key, when the answer is a choice.
    #[must_use]
    pub fn choice(&self, key: AnswerKey) -> Option<&str> {
        match self.get(key)? {
            Answer::Choice(choice) => Some(choice),
            _ => None, // Any other kind of answer is no choice.
        }
    }

    /// Whether the confirmation under a key was accepted.
    #[must_use]
    pub fn is_confirmed(&self, key: AnswerKey) -> bool {
        matches!(self.get(key), Some(Answer::Confirmed))
    }

    /// The reflog read under a key.
    #[must_use]
    pub fn reflog(&self, key: AnswerKey) -> Option<&[ReflogEntry]> {
        match self.get(key)? {
            Answer::Reflog(entries) => Some(entries),
            _ => None, // Any other kind of answer is no reflog.
        }
    }

    /// The entry of the journal read under a key: `None` when it was not read yet, `Some(None)` when there is none.
    #[must_use]
    pub fn journal(&self, key: AnswerKey) -> Option<Option<&JournalEntry>> {
        match self.get(key)? {
            Answer::Journal(entry) => Some(entry.as_ref()),
            _ => None, // Any other kind of answer is no entry of the journal.
        }
    }

    /// The commit a name read under a key stands for: `None` when it was not read yet, `Some(None)` when git knows
    /// no commit by that name.
    #[must_use]
    pub fn commit(&self, key: AnswerKey) -> Option<Option<&CommitId>> {
        match self.get(key)? {
            Answer::Commit(commit) => Some(commit.as_ref()),
            _ => None, // Any other kind of answer is no commit.
        }
    }

    /// The yes or no of a query under a key.
    #[must_use]
    pub fn flag(&self, key: AnswerKey) -> Option<bool> {
        match self.get(key)? {
            Answer::Flag(flag) => Some(*flag),
            _ => None, // Any other kind of answer is no flag.
        }
    }
}
