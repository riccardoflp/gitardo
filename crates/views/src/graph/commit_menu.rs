//! The context menu of a commit of the timeline, and of a label on it: the operations of the branch, the remote branch
//! or the tag the label stands for, those of the commit, and the fold of a merge; and what a double click on a branch
//! label does (`branch.open`).

use model::CommitId;
use model::branch::OpenedReference;
use model::commit::Commit;
use model::graph::{Label, LabelKind};
use ui::{MenuAnchor, MenuEntry, Ui};

use super::fold::{MergeEntry, merge_menu};
use super::intent::GraphIntent;
use super::state::HistoryState;
use crate::branch::{
    OperationEntry, ReferenceMenus, branch_entries, chosen, commit_entries, menu_items, open_intent, remote_entries,
    tag_entries,
};
use crate::intent::Intent;

/// The label the pointer is on: its commit, and its place among the labels of the commit.
pub(crate) type PointedLabel = (CommitId, usize);

/// What an entry of the menu of a commit does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CommitCommand {
    /// The operation at a place among those of the label and of the commit.
    Operation(usize),
    /// The fold of a merge.
    Merge(MergeEntry),
    /// **Copy `<upstream>`** on a label a branch shares with its upstream: the name of the upstream on the clipboard.
    CopyUpstream,
}

/// The key of the menu of a row, and of the label on it it was opened on.
pub(super) fn menu_key(shown: usize, label: Option<usize>) -> (&'static str, usize, Option<usize>) {
    ("row menu", shown, label)
}

/// Opens the menu of a row where it was right clicked: the menu of the branch, the remote branch or the tag the
/// pointer is on, when it is on one.
pub(super) fn open_row_menu(state: &HistoryState<'_>, ui: &mut Ui<'_>, shown: usize, pointed: Option<&PointedLabel>) {
    let label = pointed
        .filter(|pointed| {
            pointed_label(state, pointed).is_some_and(|label| {
                matches!(
                    label.kind,
                    LabelKind::Branch { .. } | LabelKind::RemoteBranch | LabelKind::Tag
                )
            })
        })
        .map(|(_, index)| *index);
    ui.open_menu(menu_key(shown, label), MenuAnchor::Pointer);
}

/// What a double click on a label asks, when it is on a branch or a remote branch: what `branch.open` decides, which
/// may be nothing. None when it is on no such label, and the row opens as it does.
pub(super) fn double_click_label(state: &HistoryState<'_>, pointed: Option<&PointedLabel>) -> Option<Vec<Intent>> {
    let label = pointed_label(state, pointed?)?;
    let reference = match label.kind {
        LabelKind::Branch { .. } => OpenedReference::Branch(&label.name),
        LabelKind::RemoteBranch => OpenedReference::RemoteBranch(&label.name),
        LabelKind::Head | LabelKind::Tag | LabelKind::Stash { .. } => return None,
    };
    Some(
        state
            .references
            .and_then(|menus| open_intent(menus, reference))
            .into_iter()
            .collect(),
    )
}

/// The label of a commit at a place.
fn pointed_label<'a>(state: &HistoryState<'a>, (commit, index): &PointedLabel) -> Option<&'a Label> {
    state.layout.labels.get(commit)?.get(*index)
}

/// The words of **Copy `<upstream>`** and the name it copies, on a label a branch shares with its upstream
/// (`graph.combine-upstream`).
pub(crate) fn copy_upstream(label: &Label) -> Option<(String, &str)> {
    match &label.kind {
        LabelKind::Branch {
            upstream: Some(upstream),
            ..
        } => Some((crate::text::timeline::copy_name(upstream), upstream.as_str())),
        _ => None,
    }
}

/// The operations of the label of a commit at a place, as its reference has them in the branch rail.
fn label_entries(state: &HistoryState<'_>, menus: ReferenceMenus<'_>, pointed: &PointedLabel) -> Vec<OperationEntry> {
    let Some(label) = pointed_label(state, pointed) else {
        return Vec::new();
    };
    let repository = menus.repository;
    match label.kind {
        LabelKind::Branch { .. } => repository
            .branches
            .iter()
            .find(|branch| branch.name == label.name)
            .map(|branch| branch_entries(menus, branch)),
        LabelKind::RemoteBranch => repository
            .remote_branches
            .iter()
            .find(|remote| remote.name == label.name)
            .map(|remote| remote_entries(menus, remote)),
        LabelKind::Tag => repository
            .tags
            .iter()
            .find(|tag| tag.name == label.name)
            .map(|tag| tag_entries(menus, tag)),
        LabelKind::Head | LabelKind::Stash { .. } => None,
    }
    .unwrap_or_default()
}

/// The menu of a commit while it is open, on the commit or on one of its labels, and what the entry chosen asks.
pub(super) fn commit_menu(state: &HistoryState<'_>, ui: &mut Ui<'_>, shown: usize, commit: &Commit) -> Vec<Intent> {
    let labels = state.layout.labels.get(&commit.id).map_or(0, Vec::len);
    let Some(label) = std::iter::once(None)
        .chain((0..labels).map(Some))
        .find(|label| ui.is_menu_open(menu_key(shown, *label)))
    else {
        return Vec::new();
    };
    let mut operations = Vec::new();
    let mut groups = Vec::new();
    if let Some(menus) = state.references {
        if let Some(label) = label {
            let entries = label_entries(state, menus, &(commit.id.clone(), label));
            groups.push(entries.len());
            operations.extend(entries);
        }
        operations.extend(commit_entries(menus, &commit.id));
    }
    let mut entries = grouped(&operations, &groups);
    let merge = merge_menu(
        state.layout.foldable.contains(&commit.id),
        state.folds.folded.contains(&commit.id),
    );
    if !merge.is_empty() && !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.extend(merge.into_iter().map(|entry| match entry {
        MenuEntry::Item(item) => MenuEntry::Item(ui::MenuItem::new(item.label, CommitCommand::Merge(item.value))),
        MenuEntry::Submenu { .. } | MenuEntry::Separator => MenuEntry::Separator,
    }));
    let shared = label
        .and_then(|label| pointed_label(state, &(commit.id.clone(), label)))
        .and_then(copy_upstream);
    if let Some((words, _)) = &shared {
        if !entries.is_empty() {
            entries.push(MenuEntry::Separator);
        }
        entries.push(MenuEntry::Item(ui::MenuItem::new(
            words.as_str(),
            CommitCommand::CopyUpstream,
        )));
    }
    match ui.menu(menu_key(shown, label), &entries) {
        Some(CommitCommand::Operation(index)) => chosen(&operations, index).into_iter().collect(),
        Some(CommitCommand::Merge(MergeEntry::Fold)) => vec![Intent::Graph(GraphIntent::Fold(commit.id.clone()))],
        Some(CommitCommand::Merge(MergeEntry::Unfold)) => vec![Intent::Graph(GraphIntent::Unfold(commit.id.clone()))],
        Some(CommitCommand::CopyUpstream) => {
            if let Some((_, name)) = shared {
                ui.copy_to_clipboard(name.to_owned());
            }
            Vec::new()
        }
        None => Vec::new(),
    }
}

/// The items of the operations, with a separator after each group of the sizes given that is followed by more.
fn grouped<'a>(operations: &'a [OperationEntry], groups: &[usize]) -> Vec<MenuEntry<'a, CommitCommand>> {
    let mut entries = Vec::new();
    let mut start = 0_usize;
    for size in groups.iter().copied().chain(std::iter::once(usize::MAX)) {
        let end = start.saturating_add(size).min(operations.len());
        if !entries.is_empty() && end > start {
            entries.push(MenuEntry::Separator);
        }
        let items = menu_items(operations.get(start..end).unwrap_or_default(), |index| {
            CommitCommand::Operation(start + index)
        });
        entries.extend(items);
        start = end;
    }
    entries
}
