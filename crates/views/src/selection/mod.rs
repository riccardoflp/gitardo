//! Several commits of the timeline selected at once (`selection.select`), the bar over the commits that counts them
//! and offers what is done most with them (`selection.bar`), and the context menu of all of them (`selection.menu`).

mod view;

pub(crate) use view::{bar_height, bar_view, menu_view, open_menu, select_view};

use model::CommitId;
use model::contract::{FeatureId, Target};
use model::graph::{Row, ShownRow};
use ui::Modifiers;

use crate::graph::{GraphIntent, HistoryState, Selected};
use crate::text::history as words;
use crate::text::patch as patch_words;

/// What a click on a row of the timeline does to the selection, by the keys held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Click {
    /// A plain click: that row alone.
    Plain,
    /// `Ctrl`+click, `Cmd`+click on macOS: adds the commit, or takes it out.
    Toggle,
    /// `Shift`+click: every commit from the anchor to the one clicked.
    Extend,
}

impl Click {
    /// The click the keys held make: on macOS, `Cmd` does what `Ctrl` does elsewhere.
    pub(crate) fn of(modifiers: Modifiers, is_mac: bool) -> Click {
        let toggles = modifiers.command() || (is_mac && modifiers.contains(Modifiers::META));
        if modifiers.contains(Modifiers::SHIFT) && !toggles {
            Click::Extend
        } else if toggles && !modifiers.contains(Modifiers::SHIFT) {
            Click::Toggle
        } else {
            Click::Plain
        }
    }
}

/// The commits of a selection of several, in the order of the timeline, newest first, as they are loaded and shown;
/// none while fewer than two are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Several {
    /// The commits, newest first.
    pub(crate) commits: Vec<CommitId>,
    /// Whether a merge commit is among them.
    pub(crate) has_merge: bool,
}

impl Several {
    /// The commits selected that the timeline shows as commits of their own, in its order; none under two.
    pub(crate) fn of(state: &HistoryState<'_>) -> Option<Several> {
        let mut shown: Vec<(usize, &model::commit::Commit)> = state
            .several
            .iter()
            .filter_map(|commit| {
                let place = *state.layout.shown_of.get(commit)?;
                commit_at(state, place)
                    .filter(|row| &row.id == commit)
                    .map(|row| (place, row))
            })
            .collect();
        if shown.len() < 2 {
            return None;
        }
        shown.sort_by_key(|(place, _)| *place);
        Some(Several {
            has_merge: shown.iter().any(|(_, commit)| commit.is_merge()),
            commits: shown.into_iter().map(|(_, commit)| commit.id.clone()).collect(),
        })
    }

    /// Whether a row shown is one of them.
    pub(crate) fn contains_row(&self, state: &HistoryState<'_>, place: usize) -> bool {
        commit_at(state, place).is_some_and(|commit| self.commits.contains(&commit.id))
    }
}

/// The commit of a row shown, when it is a commit of the history: not the draft, a stash or a folded row.
fn commit_at<'a>(state: &HistoryState<'a>, place: usize) -> Option<&'a model::commit::Commit> {
    match state.layout.folded.rows.get(place)? {
        ShownRow::Row { index, .. } => match state.rows.get(*index)? {
            Row::Commit(commit) => Some(commit),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        },
        ShownRow::Fold(_) => None,
    }
}

/// What a click on a row shown asks of the selection, by the keys held: the rules of `selection.select`. A plain click,
/// or any click on the draft or a stash, selects that row alone (`None`: the timeline selects it as it always does).
pub(crate) fn clicked(state: &HistoryState<'_>, place: usize, click: Click) -> Option<GraphIntent> {
    let commit = commit_at(state, place)?;
    let selected: Vec<CommitId> = if state.several.is_empty() {
        match state.selected {
            Some(Selected::Commit(alone)) => state
                .layout
                .shown_of
                .get(alone)
                .and_then(|place| commit_at(state, *place))
                .map(|row| vec![row.id.clone()])
                .unwrap_or_default(),
            Some(Selected::Draft) | None => Vec::new(),
        }
    } else {
        state.several.to_vec()
    };
    let chosen = match click {
        Click::Plain => return None,
        Click::Toggle => toggled(selected, &commit.id),
        Click::Extend => {
            let anchor = selected
                .first()
                .and_then(|anchor| state.layout.shown_of.get(anchor).copied())?;
            spanned(state, anchor, place)
        }
    };
    Some(match chosen.as_slice() {
        [] => GraphIntent::Select(Selected::Commit(commit.id.clone())),
        [alone] => GraphIntent::Select(Selected::Commit(alone.clone())),
        [..] => {
            let shown = if chosen.contains(&commit.id) {
                commit.id.clone()
            } else {
                chosen.last().cloned().unwrap_or_else(|| commit.id.clone())
            };
            GraphIntent::SelectSeveral { commits: chosen, shown }
        }
    })
}

/// The selection with a commit added at its end, or taken out when it was in it.
fn toggled(mut selected: Vec<CommitId>, commit: &CommitId) -> Vec<CommitId> {
    if let Some(place) = selected.iter().position(|kept| kept == commit) {
        let _ = selected.remove(place);
    } else {
        selected.push(commit.clone());
    }
    selected
}

/// The commits shown from the anchor to a row, both included, the anchor first then in the order of the timeline,
/// leaving out the draft, the stashes and the folded rows.
fn spanned(state: &HistoryState<'_>, anchor: usize, place: usize) -> Vec<CommitId> {
    let rows = anchor.min(place)..=anchor.max(place);
    let anchor_commit = commit_at(state, anchor).map(|commit| commit.id.clone());
    anchor_commit
        .clone()
        .into_iter()
        .chain(
            rows.filter_map(|row| commit_at(state, row).map(|commit| commit.id.clone()))
                .filter(|commit| Some(commit) != anchor_commit.as_ref()),
        )
        .collect()
}

/// What an entry of the bar or of the menu of a selection does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectionAction {
    /// Runs an operation on the commits selected, newest first, and empties the selection.
    Run(FeatureId),
    /// Runs an operation on the commits selected, newest first, which stay selected: a copy or a save.
    Keep(FeatureId),
    /// Opens the comparison of the two commits selected, the older (the lower in the timeline) first
    /// (`compare.with`).
    CompareTwo,
    /// Copies the full hashes, one per line, newest first.
    CopyHashes,
    /// Empties the selection.
    Clear,
}

/// An entry of the bar or of the menu of a selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectionEntry {
    /// What it says.
    pub(crate) label: String,
    /// What its tooltip says, in the bar.
    pub(crate) tooltip: Option<&'static str>,
    /// What it does.
    pub(crate) action: SelectionAction,
}

impl SelectionEntry {
    /// An entry that runs an operation on the commits selected.
    fn runs(label: String, tooltip: Option<&'static str>, feature: &'static str) -> SelectionEntry {
        SelectionEntry {
            label,
            tooltip,
            action: SelectionAction::Run(FeatureId(feature)),
        }
    }

    /// An entry that does something to the selection itself.
    fn does(label: &str, action: SelectionAction) -> SelectionEntry {
        SelectionEntry {
            label: label.to_owned(),
            tooltip: None,
            action,
        }
    }

    /// An entry that runs an operation on the commits selected and leaves them selected.
    fn keeps(label: &str, tooltip: Option<&'static str>, feature: &'static str) -> SelectionEntry {
        SelectionEntry {
            label: label.to_owned(),
            tooltip,
            action: SelectionAction::Keep(FeatureId(feature)),
        }
    }

    /// Whether it runs an operation, and is disabled while one runs.
    pub(crate) fn runs_an_operation(&self) -> bool {
        matches!(
            self.action,
            SelectionAction::Run(_) | SelectionAction::Keep(_) | SelectionAction::CompareTwo
        )
    }
}

/// What is checked out, as the entries say it: the branch, or `HEAD` when detached.
fn current<'a>(state: &HistoryState<'a>) -> &'a str {
    state
        .references
        .and_then(|menus| menus.repository.checked_out_branch())
        .unwrap_or("HEAD")
}

/// Whether `selection.cherry-pick` is offered for the commits: none of them is a merge, and no merge, rebase,
/// cherry-pick, revert or patch is under way.
fn offers_cherry_pick(state: &HistoryState<'_>, several: &Several) -> bool {
    !several.has_merge
        && state
            .references
            .is_some_and(|menus| menus.repository.operation.is_none())
}

/// Whether `squash.commits` is offered for the commits: two or more that end the line to `HEAD` of the oldest of them,
/// with none of that line left out, and no operation under way.
fn offers_squash(state: &HistoryState<'_>, several: &Several) -> bool {
    state
        .references
        .is_some_and(|menus| crate::rebase::can_squash(menus.repository, state.rows, &several.commits))
}

/// The buttons of the bar, in order, after **N commits selected** and its hint.
pub(crate) fn bar_entries(state: &HistoryState<'_>, several: &Several) -> Vec<SelectionEntry> {
    let mut entries = Vec::new();
    if offers_squash(state, several) {
        entries.push(SelectionEntry::runs(
            words::SQUASH_INTO_ONE.to_owned(),
            Some(words::SQUASH_SELECTION_TOOLTIP),
            "squash.commits",
        ));
    }
    if offers_cherry_pick(state, several) {
        entries.push(SelectionEntry::runs(
            words::cherry_pick_onto(current(state)),
            Some(words::CHERRY_PICK_SELECTION_TOOLTIP),
            "selection.cherry-pick",
        ));
    }
    if !several.has_merge {
        entries.push(SelectionEntry::keeps(
            patch_words::COPY_AS_PATCH,
            Some(patch_words::COPY_SELECTION_TOOLTIP),
            "patch.copy",
        ));
    }
    entries.push(SelectionEntry::does(words::CLEAR_SELECTION, SelectionAction::Clear));
    entries
}

/// The entries of the menu of a selection, in order, with the place of the separator before **Copy hashes**.
pub(crate) fn menu_entries(state: &HistoryState<'_>, several: &Several) -> (Vec<SelectionEntry>, usize) {
    let mut entries = Vec::new();
    if offers_squash(state, several) {
        entries.push(SelectionEntry::runs(
            words::squash_commits_into_one(several.commits.len()),
            None,
            "squash.commits",
        ));
    }
    if offers_cherry_pick(state, several) {
        entries.push(SelectionEntry::runs(
            words::cherry_pick_commits_onto(several.commits.len(), current(state)),
            None,
            "selection.cherry-pick",
        ));
    }
    if several.commits.len() == 2 {
        entries.push(SelectionEntry::does(words::COMPARE_TWO, SelectionAction::CompareTwo));
    }
    let separator = entries.len();
    entries.push(SelectionEntry::does(words::COPY_HASHES, SelectionAction::CopyHashes));
    if !several.has_merge {
        entries.push(SelectionEntry::keeps(patch_words::COPY_AS_PATCH, None, "patch.copy"));
        entries.push(SelectionEntry::keeps(patch_words::SAVE_AS_PATCH, None, "patch.save"));
    }
    (entries, separator)
}

/// What an entry asks, for the commits selected.
pub(crate) fn entry_intents(entry: &SelectionEntry, several: &Several) -> Vec<crate::intent::Intent> {
    use crate::feature::FeatureIntent;
    use crate::intent::Intent;
    match &entry.action {
        SelectionAction::Run(feature) => vec![
            Intent::Feature(FeatureIntent::Run(*feature, Target::Commits(several.commits.clone()))),
            Intent::Graph(GraphIntent::ClearSelection),
        ],
        SelectionAction::Keep(feature) => vec![Intent::Feature(FeatureIntent::Run(
            *feature,
            Target::Commits(several.commits.clone()),
        ))],
        SelectionAction::CompareTwo => compare_two(several).into_iter().collect(),
        SelectionAction::CopyHashes => vec![Intent::Graph(GraphIntent::CopyHashes(several.commits.clone()))],
        SelectionAction::Clear => vec![Intent::Graph(GraphIntent::ClearSelection)],
    }
}

/// `compare.with` on the older of two commits selected, with the newer as the side it is compared with: the page opens
/// with no question asked.
fn compare_two(several: &Several) -> Option<crate::intent::Intent> {
    use crate::feature::FeatureIntent;
    use crate::intent::Intent;
    use model::contract::{Answer, AnswerKey, Answers};
    let [newer, older] = several.commits.as_slice() else {
        return None;
    };
    let answers = Answers::new()
        .with(AnswerKey("second"), Answer::Choice(String::from("revision")))
        .with(AnswerKey("second-revision"), Answer::Text(newer.as_str().to_owned()));
    Some(Intent::Feature(FeatureIntent::RunWith(
        FeatureId("compare.with"),
        Target::Commit(older.clone()),
        answers,
    )))
}
