//! The messages of the journal, from stage 3: `journal/last`, the newest entry Undo would take back, for its label; and
//! `journal/record`, an operation the extension ran in TypeScript, with what Undo needs of it
//! (`docs/transition/02-protocol.md`, Undo and the journal).

use model::CommitId;
use model::contract::{JournalEntry, Text, UndoRecord};
use model::state::Head;
use serde::{Deserialize, Serialize};

use crate::feature::NotACommit;
use crate::state::HeadJson;

/// `journal/last`: the newest entry of the journal of a repository that still holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalLastParams {
    /// The repository.
    pub repository: String,
}

/// The answer of `journal/last`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalLastResult {
    /// The entry; none when Undo goes by the reflog.
    pub entry: Option<JournalEntryJson>,
}

/// An entry of the journal, as the label of Undo needs it: the record stays in the engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalEntryJson {
    /// Its number.
    pub number: u64,
    /// What the operation did, its title.
    pub title: String,
    /// Whether Undo can take it back: false for a push, a fetch, a tag, a stash or a worktree.
    pub undoable: bool,
    /// What was checked out once it ran.
    pub head: HeadJson,
}

impl From<&JournalEntry> for JournalEntryJson {
    fn from(entry: &JournalEntry) -> JournalEntryJson {
        JournalEntryJson {
            number: entry.number,
            title: entry.title.to_string(),
            undoable: !matches!(entry.record, UndoRecord::NotUndoable { .. }),
            head: head_json(&entry.head),
        }
    }
}

/// `journal/record`: an operation the extension ran in TypeScript, so that Undo finds the newest operation whichever of
/// the two ran it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecordParams {
    /// The repository.
    pub repository: String,
    /// The title of the operation.
    pub title: String,
    /// What Undo needs of it; none when it changed only files and the index.
    pub record: Option<UndoRecordJson>,
    /// What was checked out before it ran.
    pub before: HeadJson,
    /// What was checked out once it ran.
    pub after: HeadJson,
}

/// The answer of `journal/record`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalRecordResult {
    /// The number of the entry made; none when it made none.
    pub number: Option<u64>,
}

/// What Undo needs of an operation (`UndoRecord` of `model`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UndoRecordJson {
    /// A move of `HEAD` or of the branch, taken back by the reflog.
    Reflog,
    /// What Undo cannot take back, and why.
    NotUndoable {
        /// Why.
        reason: String,
    },
    /// A branch created and not checked out.
    BranchCreated {
        /// Its name.
        name: String,
    },
    /// A branch deleted.
    BranchDeleted {
        /// Its name.
        name: String,
        /// The commit it was at.
        commit: String,
    },
    /// A stash dropped.
    StashDropped {
        /// Its message.
        message: String,
        /// Its commit.
        commit: String,
    },
    /// The conflicts of these files resolved.
    ConflictsResolved {
        /// The files.
        files: Vec<String>,
    },
    /// A squash merge.
    SquashMerge,
    /// A series of commits made at once.
    Series {
        /// Where `HEAD` was before it.
        head_before: String,
    },
    /// The last commit split.
    SplitLast {
        /// The commit split.
        commit: String,
    },
}

impl TryFrom<UndoRecordJson> for UndoRecord {
    type Error = NotACommit;

    fn try_from(record: UndoRecordJson) -> Result<UndoRecord, NotACommit> {
        let commit = |text: &str| CommitId::parse(text).ok_or_else(|| NotACommit(text.to_owned()));
        Ok(match record {
            UndoRecordJson::Reflog => UndoRecord::Reflog,
            UndoRecordJson::NotUndoable { reason } => UndoRecord::NotUndoable {
                reason: Text::from(reason),
            },
            UndoRecordJson::BranchCreated { name } => UndoRecord::BranchCreated { name },
            UndoRecordJson::BranchDeleted { name, commit: at } => UndoRecord::BranchDeleted {
                name,
                commit: commit(&at)?,
            },
            UndoRecordJson::StashDropped { message, commit: at } => UndoRecord::StashDropped {
                message,
                commit: commit(&at)?,
            },
            UndoRecordJson::ConflictsResolved { files } => UndoRecord::ConflictsResolved { files },
            UndoRecordJson::SquashMerge => UndoRecord::SquashMerge,
            UndoRecordJson::Series { head_before } => UndoRecord::Series {
                head_before: commit(&head_before)?,
            },
            UndoRecordJson::SplitLast { commit: at } => UndoRecord::SplitLast { commit: commit(&at)? },
        })
    }
}

/// What is checked out, from its JSON; a commit that is not a full id is refused.
///
/// # Errors
///
/// When the commit is not 40 or 64 hexadecimal digits.
pub fn head_of(head: &HeadJson) -> Result<Head, NotACommit> {
    let commit = match &head.commit {
        Some(text) => Some(CommitId::parse(text).ok_or_else(|| NotACommit(text.clone()))?),
        None => None,
    };
    Ok(Head {
        commit,
        branch: head.branch.clone(),
    })
}

/// The JSON of what is checked out.
#[must_use]
pub fn head_json(head: &Head) -> HeadJson {
    HeadJson {
        commit: head.commit.as_ref().map(|commit| commit.as_str().to_owned()),
        branch: head.branch.clone(),
    }
}
