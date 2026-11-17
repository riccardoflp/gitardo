//! The timeline of the repository shown, as the store keeps it: the rows read, the draft on top, the pages loaded and
//! the one being read (`graph.load-more`), the selection, the folds, the label under the pointer and the history
//! brought out, laid out again only when one of them changes.

mod folds;
mod narrow;
mod paging;
mod sizes;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use model::CommitId;
use model::filter::BranchesShown;
use model::graph::{CellTrace, DateTime, Row, Trace};
use model::state::RepositoryState;
use ui::RevealPlace;
use views::graph::{
    DraftState, GraphIntent, HoverTarget, HoverWay, LayoutInput, Selected, TimelineLayout, TimelineSwitch,
    TraceStrength,
};

use super::message::{Effect, JobId};

use super::state::State;
pub use engine::timeline::{commits_request, history_of};

pub use narrow::{keep_shown, kept_shown, show, timeline_read};
pub use paging::{cancel_more, commits_read, known, load_up_to, read_again, read_request};
pub use sizes::{apply_switch, workspace_panes};

/// The timeline of the repository shown.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Timeline {
    /// The folder of the repository the rows are of.
    pub folder: Option<PathBuf>,
    /// The rows read, the draft first when there is one.
    pub rows: Vec<Row>,
    /// The draft, when it is the first row.
    pub draft: Option<DraftState>,
    /// Whether the history has more commits than those read.
    pub has_more: bool,
    /// How many pages are loaded.
    pub pages: usize,
    /// The read of the next pages, while it runs.
    pub more_job: Option<JobId>,
    /// The read of the first page of another history, the timeline narrowed or shown whole again, while it runs.
    pub narrow_job: Option<JobId>,
    /// The branches shown, as checked against the repository at its last read; none is everything (`filter.picker`).
    pub shown: Option<BranchesShown>,
    /// Whether the commits are read again from the first page, for another history.
    pub is_reading_again: bool,
    /// The commits loaded when the next page was asked for last.
    pub asked_at: Option<usize>,
    /// The timeline laid out.
    pub layout: Arc<TimelineLayout>,
    /// The row selected.
    pub selected: Option<Selected>,
    /// The commits selected together, the anchor first; empty while a row is selected alone (`selection.select`).
    pub several: Vec<CommitId>,
    /// The row to bring into sight, with the number that asks for it once.
    pub reveal: Option<(u64, RevealPlace)>,
    /// The label under the pointer whose history stands out.
    pub hover: Option<HoverTarget>,
    /// When the pointer left the labels.
    pub hover_left_at: Option<Duration>,
    /// The history brought out, for each row shown, and how strongly.
    pub trace: Option<(Arc<Vec<CellTrace>>, TraceStrength)>,
    /// The merges folded.
    pub folded: HashSet<CommitId>,
    /// The merges unfolded by hand while all are folded.
    pub unfolded: HashSet<CommitId>,
    /// Whether every merge is folded, also those loaded later.
    pub is_all_folded: bool,
    /// When the repository was read, in seconds since 1970.
    pub read_at: i64,
    /// What was dropped last on a row, for the menu of the drop while it is open (`drag.drop`).
    pub dropped: Option<views::drag::Dropped>,
}

impl Timeline {
    /// How many commits are loaded: the rows read but the draft.
    #[must_use]
    pub fn loaded(&self) -> usize {
        self.rows.len() - usize::from(self.draft.is_some())
    }

    /// The rows read from git, without the draft.
    #[must_use]
    pub fn read_rows(&self) -> &[Row] {
        self.rows.get(usize::from(self.draft.is_some())..).unwrap_or_default()
    }
}

/// The draft of the uncommitted changes, when there are some and the commit checked out is among the rows read, or
/// there is none yet (`graph.uncommitted-row`).
#[must_use]
pub fn draft_of(repository: &RepositoryState, rows: &[Row]) -> Option<DraftState> {
    let tree = &repository.working_tree;
    if tree.is_clean() {
        return None;
    }
    let is_listed = repository
        .head
        .commit
        .as_ref()
        .is_none_or(|head| rows.iter().any(|row| row.id() == Some(head)));
    is_listed.then(|| DraftState {
        files: tree.file_count(),
        staged: tree.staged.len(),
        unstaged: tree.unstaged.len() + tree.untracked.len(),
        conflicted: tree.conflicted.len(),
        has_commit: repository.head.commit.is_some(),
        is_operation_stopped: repository.operation.is_some(),
    })
}

/// Whether the branches shown have what is checked out: a timeline narrowed to references ticked one by one has the
/// draft only when the branch checked out is among them (`filter.show-only`).
fn shows_checked_out(shown: Option<&BranchesShown>, repository: &RepositoryState) -> bool {
    match shown {
        Some(BranchesShown::References(names)) => {
            repository.head.branch.as_ref().is_some_and(|head| names.contains(head))
        }
        _ => true,
    }
}

/// Takes the rows of a read of the repository shown, of the history of the branches shown: a read that finds the
/// same rows changes none of them; a new repository starts with nothing selected and the folds kept for it. A search
/// shown as a list, the history of a commit alone, or of a file, some lines or a folder, has no draft. Says whether the
/// rows changed.
pub fn take_read(
    state: &mut State,
    (folder, repository): (&Path, &RepositoryState),
    (rows, shown): (Vec<Row>, Option<BranchesShown>),
    page: (bool, usize, i64),
) -> bool {
    let (has_more, pages, read_at) = page;
    if state.timeline.folder.as_deref() != Some(folder) {
        state.timeline = Timeline {
            folder: Some(folder.to_path_buf()),
            ..Timeline::default()
        };
        folds::restore(state, folder);
    }
    let is_narrowed = super::search::is_narrowed(state);
    let draft = draft_of(repository, &rows).filter(|_| !is_narrowed && shows_checked_out(shown.as_ref(), repository));
    state.timeline.shown = shown;
    let mut all = Vec::with_capacity(rows.len() + 1);
    if let Some(changes) = &draft {
        all.push(Row::Uncommitted {
            parent: repository.head.commit.clone(),
            files: changes.files,
        });
    }
    all.extend(rows);
    let timeline = &mut state.timeline;
    timeline.is_reading_again = false;
    timeline.has_more = has_more;
    timeline.pages = pages;
    timeline.read_at = read_at;
    timeline.draft = draft;
    let is_changed = timeline.rows != all;
    if is_changed {
        timeline.rows = all;
        timeline.asked_at = None;
    }
    if timeline.selected == Some(Selected::Draft) && timeline.draft.is_none() {
        timeline.selected = None;
    }
    folds::fold_new_merges(state);
    relayout(state);
    is_changed
}

/// Lays the timeline out again from its rows, its folds and the repository as last read, then its history and the
/// rows a search found. The commits of a search shown as a list are not each other's history: they are laid out
/// with no lines and no folds.
pub fn relayout(state: &mut State) {
    let Some(read) = state.shown_read() else {
        return;
    };
    let switches = state.settings.switches;
    let timeline = &state.timeline;
    let is_list = state.search.list.is_some();
    let unlinked: Vec<Row> = if is_list { unlinked(&timeline.rows) } else { Vec::new() };
    let no_folds = HashSet::new();
    let input = LayoutInput {
        rows: if is_list { &unlinked } else { &timeline.rows },
        repository: &read.state,
        folded: if is_list { &no_folds } else { &timeline.folded },
        combine_upstream: switches.is_on(TimelineSwitch::CombineUpstream),
        separators_at: switches.is_on(TimelineSwitch::DateSeparators).then_some(DateTime {
            moment: model::Timestamp(timeline.read_at),
            utc_offset: 0,
        }),
        one_lane: switches.is_on(TimelineSwitch::OneLane),
    };
    let layout = TimelineLayout::new(&input);
    state.timeline.layout = Arc::new(layout);
    retrace(state);
    super::search::after_layout(state);
}

/// Rows with no parents, so that no line joins them.
fn unlinked(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .cloned()
        .map(|mut row| {
            if let Row::Commit(commit) = &mut row {
                commit.parents.clear();
            }
            row
        })
        .collect()
}

/// Works out the history brought out: the one of the label under the pointer, or else the one of the commit selected
/// while `graph.traceSelected` is on; none for the draft and while nothing is.
pub fn retrace(state: &mut State) {
    let head = state.shown_read().and_then(|read| read.state.head.commit.clone());
    let timeline = &state.timeline;
    let wanted = match (&timeline.hover, &timeline.selected) {
        (Some(hover), _) => {
            let head = head.filter(|head| timeline.layout.shown_of.contains_key(head));
            let trace = match (hover.way, head) {
                (HoverWay::NotCheckedOut, Some(other)) => Trace::HasNot {
                    commit: hover.commit.clone(),
                    other,
                },
                _ => Trace::Reaches(hover.commit.clone()),
            };
            Some((trace, TraceStrength::Label))
        }
        // Several commits selected bring out no history (`graph.trace-selected`).
        (None, Some(Selected::Commit(commit)))
            if timeline.several.len() < 2 && state.settings.switches.is_on(TimelineSwitch::TraceSelected) =>
        {
            Some((Trace::Around(commit.clone()), TraceStrength::Selected))
        }
        _ => None,
    };
    let wanted = wanted.filter(|_| state.search.list.is_none());
    let trace = wanted.and_then(|(trace, strength)| {
        timeline
            .layout
            .trace(&timeline.rows, &trace)
            .map(|cells| (Arc::new(cells), strength))
    });
    state.timeline.trace = trace;
}

/// Changes the timeline as the user asked.
pub fn reduce_graph(state: &mut State, intent: GraphIntent) -> Vec<Effect> {
    match intent {
        GraphIntent::Select(selected) => {
            select(state, selected, RevealPlace::Nearest);
            Vec::new()
        }
        GraphIntent::SelectSeveral { commits, shown } => {
            select(state, Selected::Commit(shown), RevealPlace::Nearest);
            state.timeline.several = commits;
            retrace(state);
            Vec::new()
        }
        GraphIntent::ClearSelection => {
            state.timeline.several.clear();
            retrace(state);
            Vec::new()
        }
        GraphIntent::CopyHashes(commits) => {
            let lines: Vec<&str> = commits.iter().map(CommitId::as_str).collect();
            vec![Effect::CopyText(lines.join("\n"))]
        }
        GraphIntent::GoTo(commit) => super::search::go_to(state, commit),
        GraphIntent::Open(commit) => match state.timeline.folder.clone() {
            Some(folder) => super::commit_page::open(state, folder, commit),
            None => Vec::new(),
        },
        GraphIntent::LoadMore { loaded } => paging::load_more(state, loaded),
        GraphIntent::Fold(merge) => folds::fold(state, merge),
        GraphIntent::Unfold(merge) => folds::unfold(state, &merge),
        GraphIntent::FoldAll => folds::fold_all(state),
        GraphIntent::UnfoldAll => folds::unfold_all(state),
        GraphIntent::Hover(target) => {
            state.timeline.hover = target;
            state.timeline.hover_left_at = None;
            retrace(state);
            Vec::new()
        }
        GraphIntent::HoverLeft(at) => {
            state.timeline.hover_left_at = Some(at);
            Vec::new()
        }
        GraphIntent::SetGraphWidth(_) | GraphIntent::SetPane(..) | GraphIntent::ToggleReferences => {
            sizes::resize(state, &intent)
        }
        GraphIntent::Switch(switch, is_on) => sizes::switch(state, switch, is_on),
        GraphIntent::Dropped(dropped) => {
            state.timeline.dropped = Some(dropped);
            Vec::new()
        }
    }
}

/// Selects a row alone and brings it into sight.
/// Selects a commit shown and scrolls it to the middle of the list, as going to it ends (`graph.show-in-graph`).
pub fn select_in_middle(state: &mut State, commit: CommitId) {
    select(state, Selected::Commit(commit), RevealPlace::Center);
}

fn select(state: &mut State, selected: Selected, place: RevealPlace) {
    let serial = state.timeline.reveal.map_or(1, |(serial, _)| serial + 1);
    state.timeline.several.clear();
    state.timeline.selected = Some(selected);
    state.timeline.reveal = Some((serial, place));
    retrace(state);
}
