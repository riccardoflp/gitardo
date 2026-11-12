//! Moving through the timeline with the keyboard (`graph.keyboard`): the arrows go from commit to commit, passing over
//! folded rows; `Home` goes to the commit checked out; `Alt` follows the lines of the graph; `←` and `→` fold and
//! unfold a merge; `Enter` opens the commit selected.

use model::graph::{Row, ShownRow};
use ui::{Key, KeyEvent, Modifiers, NamedKey, Ui};

use super::intent::GraphIntent;
use super::row::{select_row, selected_row};
use super::state::HistoryState;
use crate::intent::Intent;

/// The keys the list of commits leaves to the timeline: the others, such as `Page Down` and `Ctrl+Home`, are the
/// list's.
pub(crate) const LEFT_TO_VIEW: [(Key, Modifiers); 12] = [
    (Key::Named(NamedKey::ArrowDown), Modifiers::NONE),
    (Key::Named(NamedKey::ArrowUp), Modifiers::NONE),
    (Key::Named(NamedKey::Home), Modifiers::NONE),
    (Key::Named(NamedKey::ArrowDown), Modifiers::ALT),
    (Key::Named(NamedKey::ArrowUp), Modifiers::ALT),
    (Key::Named(NamedKey::ArrowDown), SHIFT_ALT),
    (Key::Named(NamedKey::ArrowUp), SHIFT_ALT),
    (Key::Named(NamedKey::ArrowLeft), Modifiers::NONE),
    (Key::Named(NamedKey::ArrowRight), Modifiers::NONE),
    (Key::Named(NamedKey::Enter), Modifiers::NONE),
    (Key::Named(NamedKey::ArrowDown), Modifiers::SHIFT),
    (Key::Named(NamedKey::ArrowUp), Modifiers::SHIFT),
];

/// `Shift` and `Alt` held together.
const SHIFT_ALT: Modifiers = Modifiers::SHIFT.with(Modifiers::ALT);

/// The keys pressed on the list of commits that it left to the timeline, each as what it asks.
///
/// Feature: graph.keyboard
pub fn keyboard_view(state: &HistoryState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let id = ui.id(super::timeline::HISTORY_KEY);
    ui.unclaimed_keys_of(id)
        .iter()
        .filter_map(|key| key_intent(state, key))
        .map(Intent::Graph)
        .collect()
}

/// What a key asks of the timeline; none for a key that does nothing where the selection is.
#[must_use]
pub fn key_intent(state: &HistoryState<'_>, key: &KeyEvent) -> Option<GraphIntent> {
    let Key::Named(named) = key.key else {
        return None;
    };
    let selected = selected_row(state);
    match (named, key.modifiers) {
        (NamedKey::ArrowDown | NamedKey::ArrowUp, Modifiers::NONE | Modifiers::SHIFT) => {
            let down = named == NamedKey::ArrowDown;
            next_row(state, selected, down, |_| true)
        }
        (NamedKey::Home, Modifiers::NONE) => state.layout.checked_out.and_then(|row| select_row(state, row)),
        (NamedKey::ArrowDown, Modifiers::ALT) => first_parent(state, selected?),
        (NamedKey::ArrowUp, Modifiers::ALT) => child(state, selected?),
        (NamedKey::ArrowDown | NamedKey::ArrowUp, modifiers) if modifiers == SHIFT_ALT => {
            let down = named == NamedKey::ArrowDown;
            next_row(state, selected, down, |row| row.parents().len() > 1)
        }
        (NamedKey::ArrowLeft | NamedKey::ArrowRight, Modifiers::NONE) => fold_key(state, selected?, named),
        (NamedKey::Enter, Modifiers::NONE) => match read_row(state, selected?)? {
            Row::Commit(commit) | Row::Stash { commit, .. } => Some(GraphIntent::Open(commit.id.clone())),
            Row::Uncommitted { .. } => None,
        },
        _ => None,
    }
}

/// The row read a row shown stands for, when it is not a folded row.
fn read_row<'a>(state: &'a HistoryState<'_>, shown: usize) -> Option<&'a Row> {
    match state.layout.folded.rows.get(shown)? {
        ShownRow::Row { index, .. } => state.rows.get(*index),
        ShownRow::Fold(_) => None,
    }
}

/// The next row down or up that can be selected and that `wanted` takes, from the one selected; with nothing
/// selected, the first.
fn next_row(
    state: &HistoryState<'_>,
    selected: Option<usize>,
    down: bool,
    wanted: impl Fn(&Row) -> bool,
) -> Option<GraphIntent> {
    let count = state.layout.len();
    let candidates: Box<dyn Iterator<Item = usize>> = match (selected, down) {
        (None, _) => Box::new(0..count),
        (Some(from), true) => Box::new(from + 1..count),
        (Some(from), false) => Box::new((0..from).rev()),
    };
    candidates
        .into_iter()
        .find(|shown| read_row(state, *shown).is_some_and(&wanted))
        .and_then(|shown| select_row(state, shown))
}

/// `Alt+↓`: the first parent of the commit selected, when it is shown.
fn first_parent(state: &HistoryState<'_>, selected: usize) -> Option<GraphIntent> {
    let parent = read_row(state, selected)?.parents().first()?;
    let shown = *state.layout.shown_of.get(parent)?;
    let _commit = read_row(state, shown)?;
    select_row(state, shown)
}

/// `Alt+↑`: the commit made on top of the one selected on its line, whose first parent it is; or else the merge that
/// took it in. The nearest above is taken.
fn child(state: &HistoryState<'_>, selected: usize) -> Option<GraphIntent> {
    let id = read_row(state, selected)?.id()?;
    let above = |is_wanted: &dyn Fn(&Row) -> bool| {
        (0..selected)
            .rev()
            .find(|shown| read_row(state, *shown).is_some_and(is_wanted))
    };
    let on_its_line = above(&|row: &Row| row.parents().first() == Some(id));
    let merge = || above(&|row: &Row| row.parents().iter().skip(1).any(|parent| parent == id));
    let shown = on_its_line.or_else(merge)?;
    select_row(state, shown)
}

/// `←` folds the merge selected, `→` unfolds it.
fn fold_key(state: &HistoryState<'_>, selected: usize, named: NamedKey) -> Option<GraphIntent> {
    let id = read_row(state, selected)?.id()?;
    let is_folded = state.folds.folded.contains(id);
    match named {
        NamedKey::ArrowLeft if !is_folded && state.layout.foldable.contains(id) => Some(GraphIntent::Fold(id.clone())),
        NamedKey::ArrowRight if is_folded => Some(GraphIntent::Unfold(id.clone())),
        _ => None,
    }
}
