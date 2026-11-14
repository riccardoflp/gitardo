//! The entries that rebase and rewrite commits, in the menu of a commit of the timeline and of a branch: where
//! `rebase.find-line-before-head` and the state of the repository offer them.

use model::CommitId;
use model::commit::Commit;
use model::contract::Target;
use model::graph::Row;
use model::rebase::{LineBeforeHeadInput, find_line_before_head};
use model::state::RepositoryState;

use crate::branch::{OperationEntry, ReferenceMenus};
use crate::text::rebase as words;

/// What the menu of a commit offers to rebase and rewrite.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct RewriteMenu {
    /// The operations, each run on the commit.
    pub(crate) operations: Vec<OperationEntry>,
    /// **Interactive rebase of N commits…**, or **Edit this commit (interactive rebase)…** on the commit checked out,
    /// which opens the interactive rebase page; none when the commit has no line to `HEAD`.
    pub(crate) todo_page: Option<String>,
}

/// An entry that runs an operation on a target.
fn runs(label: impl Into<String>, feature: &'static str, target: Target) -> OperationEntry {
    OperationEntry::runs(label, feature, target)
}

/// The commits of the rows loaded, stashes and the draft left out.
fn loaded(rows: &[Row]) -> Vec<Commit> {
    rows.iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some(commit.clone()),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        })
        .collect()
}

/// **Rebase `<current>` onto this** for a commit, a branch or a remote branch whose tip is `tip`: offered while
/// something is checked out, no operation is under way, and the tip is not the commit checked out (`rebase.onto`).
pub(crate) fn rebase_onto_entry(
    repository: &RepositoryState,
    tip: &CommitId,
    target: Target,
) -> Option<OperationEntry> {
    let head = repository.head.commit.as_ref()?;
    (head != tip && repository.operation.is_none()).then(|| {
        let current = repository.checked_out_branch().unwrap_or("HEAD");
        runs(words::rebase_onto_this(current), "rebase.onto", target)
    })
}

/// The entries of a commit that rebase and rewrite it: **Rebase `<current>` onto this**; then, when the commit has a
/// line to `HEAD` among the rows loaded, **Squash into the commit before it…** when its parent has one too, **Split
/// this commit…** when it has one parent, the fixup when it is not the commit checked out and there are changes and no
/// conflict, and the interactive rebase page. None while an operation or a bisect is under way.
pub(crate) fn rewrite_menu(menus: ReferenceMenus<'_>, rows: &[Row], commit: &CommitId) -> RewriteMenu {
    let repository = menus.repository;
    let target = || Target::Commit(commit.clone());
    let mut menu = RewriteMenu::default();
    menu.operations.extend(rebase_onto_entry(repository, commit, target()));
    let Some(head) = repository
        .head
        .commit
        .as_ref()
        .filter(|_| repository.operation.is_none() && repository.bisect.is_none())
    else {
        return menu;
    };
    let commits = loaded(rows);
    let line_of = |asked: &CommitId| {
        find_line_before_head(&LineBeforeHeadInput {
            commits: &commits,
            head: Some(head),
            commit: asked,
        })
    };
    let Some(line) = line_of(commit) else {
        return menu;
    };
    let parents = commits
        .iter()
        .find(|loaded| &loaded.id == commit)
        .map_or(&[][..], |loaded| loaded.parents.as_slice());
    if let [parent] = parents {
        if line_of(parent).is_some() {
            menu.operations
                .push(runs(words::SQUASH_INTO_PREVIOUS, "squash.into-previous", target()));
        }
        menu.operations
            .push(runs(words::SPLIT_THIS_COMMIT, "split.commit", target()));
    }
    let working_tree = &repository.working_tree;
    if commit != head && working_tree.conflicted.is_empty() {
        if !working_tree.staged.is_empty() {
            menu.operations
                .push(runs(words::FIXUP_STAGED, "fixup.commit", target()));
        } else if !working_tree.unstaged.is_empty() || !working_tree.untracked.is_empty() {
            menu.operations.push(runs(words::FIXUP_ALL, "fixup.commit", target()));
        }
    }
    menu.todo_page = Some(if commit == head {
        words::EDIT_THIS_COMMIT.to_owned()
    } else {
        words::interactive_rebase_of(line.len())
    });
    menu
}

/// Whether the commits selected can be squashed into one (`squash.commits`): two or more, no operation under way, and
/// they are the oldest commits of the line to `HEAD` of the oldest of them, with none missing.
#[must_use]
pub fn can_squash(repository: &RepositoryState, rows: &[Row], selected: &[CommitId]) -> bool {
    let Some(head) = repository.head.commit.as_ref() else {
        return false;
    };
    if selected.len() < 2 || repository.operation.is_some() || repository.bisect.is_some() {
        return false;
    }
    let commits = loaded(rows);
    let longest = selected
        .iter()
        .filter_map(|commit| {
            find_line_before_head(&LineBeforeHeadInput {
                commits: &commits,
                head: Some(head),
                commit,
            })
        })
        .max_by_key(Vec::len);
    let Some(line) = longest else {
        return false;
    };
    let oldest = line
        .get(line.len().saturating_sub(selected.len())..)
        .unwrap_or_default();
    oldest.len() == selected.len() && oldest.iter().all(|commit| selected.contains(commit))
}
