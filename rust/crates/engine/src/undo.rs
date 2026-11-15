//! The journal of what Undo needs for the operations of Gitardo that the reflog does not see (`undo.journal`).

use model::contract::{JournalEntry, Plan, Text, Undo, UndoRecord};
use model::state::{Head, RepositoryState};

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
/// The journal is a stack of what Gitardo did, newest last, and it holds while `HEAD` is what the newest entry left: once
/// `HEAD` moved by something Gitardo did not do, a commit in a terminal, every entry is dropped, so that Undo goes by the
/// reflog. Each plan that ran adds an entry: what it left for Undo, a move of `HEAD` that the reflog takes back, or the
/// reason it cannot be taken back, so that Undo says it rather than take back the operation before it. A plan that cannot
/// be taken back and changed only files and the index (staging, a save, a discard) adds nothing, and Undo goes past it.
/// A plan that is the Undo of an entry drops it. The journal keeps at most [`MOST_ENTRIES`].
///
/// Feature: undo.journal
#[must_use]
pub fn journal_after(
    journal: &Journal,
    plan: Option<&Plan>,
    state_before: &RepositoryState,
    state_after: &RepositoryState,
) -> Journal {
    let mut entries = journal.entries.clone();
    if !holds(&entries, &state_before.head) {
        entries.clear();
    }
    let mut next_number = journal.next_number;
    match plan.map(|plan| (plan, record_of(plan, state_before, state_after))) {
        Some((plan, Recorded::Entry(record))) => {
            entries.push(JournalEntry {
                number: next_number,
                title: Text::from(plan.title.as_str()),
                record,
                head: state_after.head.clone(),
            });
            next_number += 1;
        }
        Some((_, Recorded::TakesBack(number))) => {
            entries.retain(|entry| entry.number != number);
            if !holds(&entries, &state_after.head) {
                entries.clear();
            }
        }
        None | Some((_, Recorded::Nothing)) => {
            if !holds(&entries, &state_after.head) {
                entries.clear();
            }
        }
    }
    let extra = entries.len().saturating_sub(MOST_ENTRIES);
    entries.drain(..extra).for_each(drop);
    Journal { entries, next_number }
}

/// The journal once an operation ran outside the engine, in the extension, which says what Undo needs of it
/// (`journal/record`): an entry for `record`, or none when it changed only files and the index. `before` and `after` are
/// what was checked out around it. It is part of `undo.journal`, whose function is [`journal_after`].
#[must_use]
pub fn journal_recorded(
    journal: &Journal,
    title: &Text,
    record: Option<&UndoRecord>,
    before: &Head,
    after: &Head,
) -> Journal {
    let mut entries = journal.entries.clone();
    if !holds(&entries, before) {
        entries.clear();
    }
    let mut next_number = journal.next_number;
    match record {
        Some(record) => {
            entries.push(JournalEntry {
                number: next_number,
                title: title.clone(),
                record: record.clone(),
                head: after.clone(),
            });
            next_number += 1;
        }
        None if !holds(&entries, after) => entries.clear(),
        None => {}
    }
    let extra = entries.len().saturating_sub(MOST_ENTRIES);
    entries.drain(..extra).for_each(drop);
    Journal { entries, next_number }
}

/// Whether the entries hold with this checked out: it is what the newest of them left.
fn holds(entries: &[JournalEntry], head: &Head) -> bool {
    entries.last().is_none_or(|newest| newest.head == *head)
}

/// What a plan leaves in the journal.
enum Recorded {
    /// An entry.
    Entry(UndoRecord),
    /// It takes back the entry of this number.
    TakesBack(u64),
    /// Nothing: it changed only files and the index, and cannot be taken back.
    Nothing,
}

fn record_of(plan: &Plan, before: &RepositoryState, after: &RepositoryState) -> Recorded {
    match &plan.undo {
        Undo::ByReflog => Recorded::Entry(UndoRecord::Reflog),
        Undo::Journal(record) => Recorded::Entry(record.clone()),
        Undo::TakesBack(number) => Recorded::TakesBack(*number),
        Undo::Impossible(reason) if references_changed(before, after) => {
            Recorded::Entry(UndoRecord::NotUndoable { reason: reason.clone() })
        }
        Undo::Impossible(_) => Recorded::Nothing,
    }
}

/// Whether anything but the files and the index changed between two reads: what is checked out, a branch, a remote
/// branch, a tag, a stash, a worktree, a remote, the identity or a submodule's checkout.
fn references_changed(before: &RepositoryState, after: &RepositoryState) -> bool {
    before.head != after.head
        || before.remotes != after.remotes
        || before.identity != after.identity
        || references(before) != references(after)
}

/// Where each branch, remote branch, tag, stash, worktree and submodule is, in words, in the order of the read.
fn references(state: &RepositoryState) -> Vec<String> {
    let branches = state
        .branches
        .iter()
        .map(|branch| format!("branch {} {}", branch.name, branch.commit));
    let remote_branches = state
        .remote_branches
        .iter()
        .map(|branch| format!("remote branch {} {}", branch.name, branch.commit));
    let tags = state.tags.iter().map(|tag| format!("tag {} {}", tag.name, tag.commit));
    let stashes = state.stashes.iter().map(|stash| format!("stash {}", stash.commit));
    let worktrees = state.worktrees.iter().map(|worktree| {
        format!(
            "worktree {} {:?} {:?}",
            worktree.folder.display(),
            worktree.commit,
            worktree.branch
        )
    });
    let submodules = state.submodules.iter().map(|submodule| {
        format!(
            "submodule {} {:?} {:?}",
            submodule.folder, submodule.url, submodule.checkout
        )
    });
    branches
        .chain(remote_branches)
        .chain(tags)
        .chain(stashes)
        .chain(worktrees)
        .chain(submodules)
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Journal, MOST_ENTRIES, journal_after, journal_recorded};
    use model::CommitId;
    use model::contract::{Danger, Plan, Text, Undo, UndoRecord};
    use model::state::{Head, RepositoryState, Tag};

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
        // Checking out another branch at the same commit moves HEAD too.
        let mut other = at('a');
        other.head.branch = Some(String::from("refs/heads/other"));
        assert!(journal_after(&journal, None, &at('a'), &other).is_empty());
    }

    #[test]
    fn undo_journal__a_move_of_head_by_gitardo_is_an_entry_on_top_of_the_ones_before() {
        let deleted_one = journal_after(&Journal::new(), Some(&plan(deleted("old"))), &at('a'), &at('a'));
        let committed = journal_after(&deleted_one, Some(&plan(Undo::ByReflog)), &at('a'), &at('c'));
        assert_eq!(committed.entries().len(), 2);
        assert_eq!(committed.newest().map(|entry| &entry.record), Some(&UndoRecord::Reflog));
        // A read with HEAD where the commit left it keeps both.
        assert_eq!(journal_after(&committed, None, &at('c'), &at('c')).entries().len(), 2);
        // Its Undo takes HEAD back to where the deletion left it, which then holds again.
        let newest = committed.newest().map(|entry| entry.number).unwrap();
        let undone = journal_after(&committed, Some(&plan(Undo::TakesBack(newest))), &at('c'), &at('a'));
        assert_eq!(undone.entries().len(), 1);
        assert!(matches!(
            undone.newest().map(|entry| &entry.record),
            Some(UndoRecord::BranchDeleted { .. })
        ));
        // A commit in a terminal on top of the one Gitardo made: nothing holds.
        assert!(journal_after(&committed, None, &at('c'), &at('d')).is_empty());
    }

    #[test]
    fn undo_journal__what_cannot_be_undone_is_an_entry_once_a_reference_changed_and_nothing_when_only_files_did() {
        let deleted_one = journal_after(&Journal::new(), Some(&plan(deleted("old"))), &at('a'), &at('a'));
        let reason = || Undo::Impossible(Text::from("A push changes the remote, which Undo does not take back."));
        // A stage changed only the index: the journal is as it was, and Undo goes past it.
        assert_eq!(
            journal_after(&deleted_one, Some(&plan(reason())), &at('a'), &at('a')),
            deleted_one
        );
        // A tag made: the reason is kept on top.
        let mut tagged = at('a');
        tagged.tags.push(Tag {
            name: String::from("v1"),
            commit: commit('a'),
            date: model::Timestamp::default(),
        });
        let after = journal_after(&deleted_one, Some(&plan(reason())), &at('a'), &tagged);
        assert_eq!(after.entries().len(), 2);
        assert!(matches!(
            after.newest().map(|entry| &entry.record),
            Some(UndoRecord::NotUndoable { .. })
        ));
    }

    #[test]
    fn undo_journal__an_operation_the_extension_ran_is_recorded_with_what_it_says() {
        let head = |digit: char| at(digit).head;
        let journal = journal_recorded(
            &Journal::new(),
            &Text::from("Drop stash"),
            Some(&UndoRecord::StashDropped {
                message: String::from("wip"),
                commit: commit('b'),
            }),
            &head('a'),
            &head('a'),
        );
        let journal = journal_recorded(
            &journal,
            &Text::from("Commit"),
            Some(&UndoRecord::Reflog),
            &head('a'),
            &head('c'),
        );
        assert_eq!(journal.entries().len(), 2);
        // A stage, recorded with nothing, keeps them.
        let staged = journal_recorded(&journal, &Text::from("Stage a.txt"), None, &head('c'), &head('c'));
        assert_eq!(staged, journal);
        // Recorded from where something else had moved HEAD: only the new entry holds.
        let moved = journal_recorded(
            &journal,
            &Text::from("Commit"),
            Some(&UndoRecord::Reflog),
            &head('d'),
            &head('e'),
        );
        assert_eq!(moved.entries().len(), 1);
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
