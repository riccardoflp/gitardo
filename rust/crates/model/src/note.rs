//! The notes of commits: texts git keeps beside the commits, in a reference of their own, without changing them
//! (`note.read`).

use std::collections::BTreeMap;

use crate::CommitId;

/// The reference of notes git uses unless `core.notesRef` names another.
pub const DEFAULT_REFERENCE: &str = "refs/notes/commits";

/// The commits that have a note in the reference of notes git uses, as last read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notes {
    /// The reference of notes git uses: `refs/notes/commits` unless `core.notesRef` names another (Q1210).
    pub reference: String,
    /// The commit the reference is at; none while no note was ever written.
    pub tip: Option<CommitId>,
    /// Each commit with a note, and the object that holds the text of its note.
    pub noted: BTreeMap<CommitId, CommitId>,
}

impl Default for Notes {
    fn default() -> Self {
        Notes {
            reference: String::from(DEFAULT_REFERENCE),
            tip: None,
            noted: BTreeMap::new(),
        }
    }
}

impl Notes {
    /// Whether the commit has a note.
    #[must_use]
    pub fn has_note(&self, commit: &CommitId) -> bool {
        self.noted.contains_key(commit)
    }

    /// Whether the repository has a reference of notes, which **Push notes** sends.
    #[must_use]
    pub fn has_reference(&self) -> bool {
        self.tip.is_some()
    }

    /// The name of the reference without `refs/notes/`: `commits`, as the reference of a remote's notes is named after
    /// it (`note.fetch`).
    #[must_use]
    pub fn short_name(&self) -> &str {
        self.reference.strip_prefix("refs/notes/").unwrap_or(&self.reference)
    }
}

#[cfg(test)]
mod tests {
    use super::Notes;

    #[test]
    fn the_short_name_leaves_out_refs_notes() {
        let notes = Notes {
            reference: String::from("refs/notes/review"),
            ..Notes::default()
        };
        assert_eq!(notes.short_name(), "review");
        assert_eq!(Notes::default().short_name(), "commits");
    }
}
