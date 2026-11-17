//! Dragging a commit or a branch onto the timeline (`drag.drop`): a row of the timeline, a branch label on it, or a
//! branch or a remote branch of the branch rail is dragged; each row of the timeline that takes it is outlined while
//! the pointer is over it, the others say with the pointer that nothing can be dropped there; dropping it opens a menu
//! at the pointer with what `drag.list-drop-entries` offers, and choosing an entry runs its operation.

mod timeline;

use model::CommitId;
use model::branch::other_worktree;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use model::drag::{Dragged, DropEntry, DropInput, DropRow, list_drop_entries};
use model::state::RepositoryState;
use ui::{MenuEntry, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::history as words;

pub(crate) use timeline::{drop_on_row, start_row_drag};

/// The key of the menu a drop opens.
pub(crate) const DROP_MENU: &str = "drop menu";
/// The answer of `rebase.branch-onto` that names the branch dropped on a commit.
const BRANCH: AnswerKey = AnswerKey("branch");

/// What is dragged.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DragItem {
    /// A commit, by its hash.
    Commit(CommitId),
    /// A local branch, by its short name.
    Branch(String),
    /// A remote branch, by its short name: `origin/feature`.
    RemoteBranch(String),
}

impl DragItem {
    /// What the drag carries for the rows it is dragged over.
    pub(crate) fn payload(&self) -> String {
        match self {
            DragItem::Commit(commit) => format!("commit {}", commit.as_str()),
            DragItem::Branch(name) => format!("branch {name}"),
            DragItem::RemoteBranch(name) => format!("remote {name}"),
        }
    }

    /// What a drag carries, read back; none for a drag of something else, such as a line of the interactive rebase.
    fn from_payload(payload: &str) -> Option<DragItem> {
        let (kind, name) = payload.split_once(' ')?;
        match kind {
            "commit" => CommitId::parse(name).map(DragItem::Commit),
            "branch" => Some(DragItem::Branch(name.to_owned())),
            "remote" => Some(DragItem::RemoteBranch(name.to_owned())),
            _ => None,
        }
    }

    /// The words that follow the pointer: the short hash of a commit, the name of a branch.
    pub(crate) fn label(&self) -> String {
        match self {
            DragItem::Commit(commit) => commit.short().to_owned(),
            DragItem::Branch(name) | DragItem::RemoteBranch(name) => name.clone(),
        }
    }

    /// The operation's target for what is dragged.
    fn target(&self) -> Target {
        match self {
            DragItem::Commit(commit) => Target::Commit(commit.clone()),
            DragItem::Branch(name) => Target::Branch(name.clone()),
            DragItem::RemoteBranch(name) => Target::RemoteBranch(name.clone()),
        }
    }
}

/// The row of the timeline something is dropped on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Onto {
    /// The draft of the uncommitted changes.
    Draft,
    /// A commit.
    Commit(CommitId),
}

/// Something dropped on a row of the timeline, which the store keeps while the menu of the drop is open.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Dropped {
    /// What was dragged.
    pub item: DragItem,
    /// Where it was dropped.
    pub onto: Onto,
}

/// What the menu of a drop is drawn from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropState<'a> {
    /// What was dropped last, and where.
    pub dropped: Option<&'a Dropped>,
    /// The repository shown, as last read.
    pub repository: &'a RepositoryState,
}

/// The menu a drop opens at the pointer, while it is open: the entries of `drag.list-drop-entries`, and the operation
/// of the one chosen, which asks and confirms as it does from a context menu. `Escape` or a click elsewhere closes it
/// and runs nothing.
///
/// Feature: drag.drop
pub fn drop_view(state: &DropState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(dropped) = state.dropped.filter(|_| ui.is_menu_open(DROP_MENU)) else {
        return Vec::new();
    };
    let offered = drop_entries(state.repository, dropped);
    let labels: Vec<String> = offered.iter().map(|entry| words::drop_entry(&entry.label)).collect();
    let entries: Vec<MenuEntry<'_, usize>> = labels
        .iter()
        .zip(&offered)
        .enumerate()
        .map(|(index, (label, entry))| MenuEntry::Item(crate::icons::operation_item(label, entry.feature.0, index)))
        .collect();
    ui.menu(DROP_MENU, &entries)
        .and_then(|index| offered.get(index))
        .map(|entry| entry_intent(state.repository, dropped, entry))
        .into_iter()
        .collect()
}

/// What dropping something on a row offers; empty when the row does not take it, or when a branch dragged is no
/// longer there.
pub(crate) fn drop_entries(repository: &RepositoryState, dropped: &Dropped) -> Vec<DropEntry> {
    let dragged = match &dropped.item {
        DragItem::Commit(commit) => Dragged::Commit(commit),
        DragItem::Branch(name) => {
            let Some(branch) = repository.branches.iter().find(|branch| branch.name == *name) else {
                return Vec::new();
            };
            Dragged::Branch {
                name,
                tip: &branch.commit,
                is_remote: false,
                is_checked_out_elsewhere: other_worktree(repository, name).is_some(),
            }
        }
        DragItem::RemoteBranch(name) => {
            let Some(remote) = repository.remote_branches.iter().find(|remote| remote.name == *name) else {
                return Vec::new();
            };
            Dragged::Branch {
                name,
                tip: &remote.commit,
                is_remote: true,
                is_checked_out_elsewhere: false,
            }
        }
    };
    let row = match &dropped.onto {
        Onto::Draft => DropRow::Draft,
        Onto::Commit(commit) => DropRow::Commit(commit),
    };
    list_drop_entries(&DropInput {
        dragged,
        row,
        head: repository.head.commit.as_ref(),
        branch: repository.checked_out_branch(),
        is_operation_under_way: repository.operation.is_some(),
    })
}

/// The operation an entry runs: on the commit dragged, on the branch dragged, or on the commit it was dropped on;
/// `rebase.branch-onto` with the branch dragged answered.
fn entry_intent(repository: &RepositoryState, dropped: &Dropped, entry: &DropEntry) -> Intent {
    let onto = match &dropped.onto {
        Onto::Commit(commit) if repository.head.commit.as_ref() != Some(commit) => Some(commit),
        Onto::Draft | Onto::Commit(_) => None,
    };
    let feature = entry.feature;
    let run = |target| Intent::Feature(FeatureIntent::Run(feature, target));
    match (feature, onto, &dropped.item) {
        (FeatureId("rebase.branch-onto"), Some(onto), DragItem::Branch(name)) => {
            let answers = Answers::new().with(BRANCH, Answer::Choice(name.clone()));
            Intent::Feature(FeatureIntent::RunWith(feature, Target::Commit(onto.clone()), answers))
        }
        // The branch checked out dropped on another commit is rebased onto it, or reset to it.
        (_, Some(onto), _) => run(Target::Commit(onto.clone())),
        // On the row checked out, the commit is cherry-picked, the branch merged or rebased onto.
        (_, None, item) => run(item.target()),
    }
}

/// Reads what a drag carries.
pub(crate) fn dragged_item(payload: &str) -> Option<DragItem> {
    DragItem::from_payload(payload)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
