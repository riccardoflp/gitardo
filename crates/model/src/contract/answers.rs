//! What a feature has learned so far: the answers of its questions and the results of its queries.

use std::collections::BTreeMap;

use crate::CommitId;
use crate::state::Identity;

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
    /// The positions of a reference in its reflog, newest first, with their selectors and times.
    Positions(Vec<crate::reflog::ReflogPosition>),
    /// The answer of a query that says yes or no.
    Flag(bool),
    /// The commit a name stands for, or none when git knows no commit by it.
    Commit(Option<CommitId>),
    /// The newest entry of the journal that still holds, if any.
    Journal(Option<JournalEntry>),
    /// How many commits each of two revisions has that the other has not.
    Counts {
        /// The commits only the left revision has.
        left: u32,
        /// The commits only the right revision has.
        right: u32,
    },
    /// Names read: the branches merged into a revision.
    Names(Vec<String>),
    /// The commits a force push would throw away (`sync.read-overwritten`).
    Overwritten(crate::remote::Overwritten),
    /// The remotes, each with its URL (`remote.read-remotes`).
    Remotes(Vec<crate::remote::Remote>),
    /// What is read of a stash before it is applied (`stash.read-files`, and the conflicts of `stash.preview`).
    Stash(crate::stash::StashReading),
    /// Who commits are made as, and the `user.*` keys the repository sets for itself.
    Identity(IdentityKeys),
    /// A file in a commit, in the index and on disk.
    FileVersions(crate::diff::FileVersions),
    /// The files a merge would leave in conflict (`merge.read-predicted-conflicts`), or none when git cannot tell.
    Conflicts(Option<Vec<String>>),
    /// Commits read, each with its parents and its subject, in the order of the query.
    Commits(Vec<crate::commit::Commit>),
    /// The whole message of a commit.
    Message(String),
    /// The commit a cherry-pick or a revert stopped on with nothing left to do, when it did
    /// (`cherry-pick.skip-empty`).
    EmptyStop(Option<super::EmptyStop>),
    /// The parents of a commit and the line from it up to `HEAD` (`Query::Line`).
    Line(crate::rebase::LineToHead),
    /// The whole messages of commits, in the order asked.
    Messages(Vec<String>),
    /// Commits as one patch made by `git format-patch`, oldest first (`patch.read-format-patch`).
    Patch(String),
    /// What applying a patch with `--3way` would do (`patch.check`).
    PatchCheck(crate::patch::PatchCheck),
    /// The bytes of a file, or the changes of a comparison as a patch (`compare.read-patch`).
    Contents(Vec<u8>),
    /// The text on the clipboard of the system; none when it holds no text.
    Clipboard(Option<String>),
    /// A path: the file chosen in a dialog of the system, or a temporary file.
    Path(std::path::PathBuf),
    /// The comparisons opened before in the repository, the latest first (`compare.recent`).
    Recent(Vec<crate::compare::RecentComparison>),
    /// Which versions the index keeps of a file in conflict, or none when it is not in conflict
    /// (`conflicts.read-files`).
    Stages(Option<crate::conflicts::FileStages>),
    /// The files in conflict on disk and in the three versions of the index (`conflicts.resolve-by-rules`).
    ConflictTexts(Vec<crate::conflicts::ConflictTexts>),
    /// The bytes of files, in the order asked, each none when it is not there.
    Files(Vec<Option<Vec<u8>>>),
}

/// Who commits are made as in a repository, and which of the `user.*` keys it sets for itself (`identity.read`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentityKeys {
    /// The name and the email git will use, wherever they are set.
    pub identity: Identity,
    /// The `user.*` keys of the repository's own configuration, each once: `user.name`, `user.signingkey`.
    pub local_keys: Vec<String>,
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

    /// Who commits are made as, read under a key.
    #[must_use]
    pub fn identity(&self, key: AnswerKey) -> Option<&IdentityKeys> {
        match self.get(key)? {
            Answer::Identity(keys) => Some(keys),
            _ => None, // Any other kind of answer is no identity.
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

    /// The choices ticked for a key, when the answer takes many.
    #[must_use]
    pub fn choices(&self, key: AnswerKey) -> Option<&[String]> {
        match self.get(key)? {
            Answer::Choices(choices) => Some(choices),
            _ => None, // Any other kind of answer is no list of choices.
        }
    }

    /// The counts read under a key: the commits only the left revision has, then those only the right one has.
    #[must_use]
    pub fn counts(&self, key: AnswerKey) -> Option<(u32, u32)> {
        match self.get(key)? {
            Answer::Counts { left, right } => Some((*left, *right)),
            _ => None, // Any other kind of answer is no counts.
        }
    }

    /// The names read under a key.
    #[must_use]
    pub fn names(&self, key: AnswerKey) -> Option<&[String]> {
        match self.get(key)? {
            Answer::Names(names) => Some(names),
            _ => None, // Any other kind of answer is no names.
        }
    }
}
