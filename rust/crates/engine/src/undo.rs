//! The journal of what Undo needs for the operations of Gitardo that the reflog does not see (`undo.journal`).

use model::contract::{JournalEntry, Plan, Text, Undo};
use model::state::RepositoryState;

/// How many entries the journal keeps for a repository: the oldest go first.
pub const MOST_ENTRIES: usize = 50;

/// The journal of one repository: its entries, oldest first. It is kept in memory only (open question Q4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Journal {
    entries: Vec<JournalEntry>,
    next_number: u64,
}

impl Journal {
    /// An empty journal.
    #[must_use]
    pub fn new() -> Self {
        Journal::default()
    }

    /// Its entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// The newest entry, the one Undo takes back first.
    #[must_use]
    pub fn newest(&self) -> Option<&JournalEntry> {
        self.entries.last()
    }

    /// Whether it has no entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The journal once a plan ran, or once the repository was read again with no plan (`plan` is then `None`), the state
/// being what was read before and after.
///
/// An entry holds only while what is checked out is what it was once its plan ran: once `HEAD` moved by something the
/// entry did not do, it is dropped, so that Undo falls back on the reflog. A plan that is the Undo of an entry drops it;
/// a plan that leaves a record for Undo adds an entry. The journal keeps at most [`MOST_ENTRIES`].
///
/// Feature: undo.journal
#[must_use]
pub fn journal_after(
    journal: &Journal,
    plan: Option<&Plan>,
    state_before: &RepositoryState,
    state_after: &RepositoryState,
) -> Journal {
    let mut entries: Vec<JournalEntry> = journal
        .entries
        .iter()
        .filter(|entry| entry.head == state_before.head && entry.head == state_after.head)
        .filter(
            |entry| !matches!(plan.map(|plan| &plan.undo), Some(Undo::TakesBack(number)) if *number == entry.number),
        )
        .cloned()
        .collect();
    let mut next_number = journal.next_number;
    if let Some(Plan {
        title,
        undo: Undo::Journal(record),
        ..
    }) = plan
    {
        entries.push(JournalEntry {
            number: next_number,
            title: Text::from(title.as_str()),
            record: record.clone(),
            head: state_after.head.clone(),
        });
        next_number += 1;
    }
    let extra = entries.len().saturating_sub(MOST_ENTRIES);
    entries.drain(..extra).for_each(drop);
    Journal { entries, next_number }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Journal, MOST_ENTRIES, journal_after};
    use model::CommitId;
    use model::contract::{Danger, Plan, Text, Undo, UndoRecord};
    use model::state::{Head, RepositoryState};

    fn commit(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap()
    }

    fn at(digit: char) -> RepositoryState {
        RepositoryState {
            head: Head {
                commit: Some(commit(digit)),
                branch: Some(String::from("refs/heads/main")),
            },
            ..RepositoryState::default()
        }
    }

    fn plan(undo: Undo) -> Plan {
        Plan {
            title: Text::from("Delete branch old"),
            steps: Vec::new(),
            danger: Danger::None,
            undo,
            after: Vec::new(),
            on_failure: Vec::new(),
        }
    }

    fn deleted(name: &str) -> Undo {
        Undo::Journal(UndoRecord::BranchDeleted {
            name: name.to_owned(),
            commit: commit('b'),
        })
    }

    #[test]
    fn undo_journal__drops_an_entry_once_head_moved_by_something_else() {
        let journal = journal_after(&Journal::new(), Some(&plan(deleted("old"))), &at('a'), &at('a'));
        assert_eq!(journal.entries().len(), 1);
        // A read with nothing moved keeps it.
        assert_eq!(journal_after(&journal, None, &at('a'), &at('a')).entries().len(), 1);
        // A commit made in a terminal moved HEAD: the entry no longer holds.
        assert!(journal_after(&journal, None, &at('a'), &at('c')).is_empty());
        // A plan of Gitardo with nothing for the journal moved HEAD: the same.
        assert!(journal_after(&journal, Some(&plan(Undo::ByReflog)), &at('a'), &at('c')).is_empty());
        // Checking out another branch at the same commit moves HEAD too.
        let mut other = at('a');
        other.head.branch = Some(String::from("refs/heads/other"));
        assert!(journal_after(&journal, None, &at('a'), &other).is_empty());
    }

    #[test]
    fn undo_journal__keeps_at_most_fifty_entries() {
        let mut journal = Journal::new();
        for index in 0..MOST_ENTRIES + 5 {
            journal = journal_after(&journal, Some(&plan(deleted(&format!("b{index}")))), &at('a'), &at('a'));
        }
        assert_eq!(journal.entries().len(), MOST_ENTRIES);
        assert_eq!(journal.entries().first().map(|entry| entry.number), Some(5));
    }

    #[test]
    fn the_undo_of_an_entry_drops_it_and_keeps_the_others() {
        let mut journal = journal_after(&Journal::new(), Some(&plan(deleted("one"))), &at('a'), &at('a'));
        journal = journal_after(&journal, Some(&plan(deleted("two"))), &at('a'), &at('a'));
        let newest = journal.newest().map(|entry| entry.number).unwrap();
        let after = journal_after(&journal, Some(&plan(Undo::TakesBack(newest))), &at('a'), &at('a'));
        assert_eq!(after.entries().len(), 1);
        assert_eq!(
            after.newest().map(|entry| entry.title.as_str()),
            Some("Delete branch old")
        );
    }
}
