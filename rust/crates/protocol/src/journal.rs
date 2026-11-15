//! The message of the journal, from stage 3: `journal/last`, the newest entry Undo would take back, for its label
//! (`docs/transition/02-protocol.md`, Undo and the journal). `journal/record`, by which the extension told the engine
//! of an operation it ran in TypeScript, went with that TypeScript in stage 10 (1.7).

use model::contract::{JournalEntry, UndoRecord};
use model::state::Head;
use serde::{Deserialize, Serialize};

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

/// The JSON of what is checked out.
#[must_use]
pub fn head_json(head: &Head) -> HeadJson {
    HeadJson {
        commit: head.commit.as_ref().map(|commit| commit.as_str().to_owned()),
        branch: head.branch.clone(),
    }
}
