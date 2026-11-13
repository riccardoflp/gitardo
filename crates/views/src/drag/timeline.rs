//! The timeline's part of a drag: a row of it starts one, of the branch whose label the button was pressed on or of
//! its commit; and each row takes what is dragged, or says that it does not.

use model::CommitId;
use model::graph::{LabelKind, Row, ShownRow};
use ui::{ListResponse, MenuAnchor, Ui};

use super::{DROP_MENU, DragItem, Dropped, Onto, dragged_item, drop_entries};
use crate::graph::HistoryState;

/// A row of the timeline while something is dragged: outlined while the pointer is over it when it takes what is
/// dragged, the pointer saying so when it does not; and what was dropped on it, which opens the menu of the drop at the
/// pointer.
pub(crate) fn drop_on_row(state: &HistoryState<'_>, ui: &mut Ui<'_>, row: &ui::Row) -> Option<Dropped> {
    let payload = ui.drag_payload()?;
    let repository = state.references.map(|menus| menus.repository);
    let accepted = onto(state, row.index)
        .zip(dragged_item(&payload))
        .map(|(onto, item)| Dropped { item, onto })
        .filter(|dropped| repository.is_some_and(|repository| !drop_entries(repository, dropped).is_empty()));
    let Some(accepted) = accepted else {
        ui.refuse_drop(row.rect);
        return None;
    };
    let _source = ui.drop_target(row.rect).dropped?;
    ui.open_menu(DROP_MENU, MenuAnchor::Pointer);
    Some(accepted)
}

/// Starts a drag from a row of the timeline when its list says one starts: of the branch or the remote branch whose
/// label the button was pressed on, or else of the commit of the row. The draft, a stash and a fold are not dragged.
pub(crate) fn start_row_drag(
    state: &HistoryState<'_>,
    ui: &mut Ui<'_>,
    (list, pointed): (&ListResponse, Option<&(CommitId, usize)>),
) {
    let Some(start) = list.drag_start else {
        return;
    };
    let item = pointed
        .and_then(|pointed| label_item(state, pointed))
        .or_else(|| commit_of(state, start.row).map(DragItem::Commit));
    if let Some(item) = item {
        ui.start_drag(&list.response, start.at, (&item.label(), &item.payload()));
    }
}

/// The branch or the remote branch of a label of a commit; none for a tag, `HEAD`, a stash or a mark of a bisect.
fn label_item(state: &HistoryState<'_>, (commit, index): &(CommitId, usize)) -> Option<DragItem> {
    let label = state.layout.labels.get(commit)?.get(*index)?;
    match label.kind {
        LabelKind::Branch { .. } => Some(DragItem::Branch(label.name.clone())),
        LabelKind::RemoteBranch => Some(DragItem::RemoteBranch(label.name.clone())),
        LabelKind::Head | LabelKind::Tag | LabelKind::Stash { .. } | LabelKind::Bisect(_) => None,
    }
}

/// The commit of a row shown, when it is a commit.
fn commit_of(state: &HistoryState<'_>, shown: usize) -> Option<CommitId> {
    match onto(state, shown)? {
        Onto::Commit(commit) => Some(commit),
        Onto::Draft => None,
    }
}

/// A row shown as something is dropped on it: the draft or a commit; none for a stash, a fold, or the last row while
/// more commits are read.
fn onto(state: &HistoryState<'_>, shown: usize) -> Option<Onto> {
    let ShownRow::Row { index, .. } = state.layout.folded.rows.get(shown)? else {
        return None;
    };
    match state.rows.get(*index)? {
        Row::Uncommitted { .. } => Some(Onto::Draft),
        Row::Commit(commit) => Some(Onto::Commit(commit.id.clone())),
        Row::Stash { .. } => None,
    }
}
