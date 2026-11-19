//! What the timeline is drawn from, made by the store of the window for each frame.

use std::collections::HashSet;
use std::time::Duration;

use model::CommitId;
use model::graph::{CellTrace, Row};
use ui::RevealPlace;

use super::intent::{HoverTarget, Selected, Switches};
use super::layout::TimelineLayout;
use crate::branch::ReferenceMenus;

/// What the timeline lists.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphState<'a> {
    /// The first commits are being read.
    Loading,
    /// The commits read, newest first.
    Listed(Box<HistoryState<'a>>),
}

/// The commits listed and everything about them the timeline draws.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryState<'a> {
    /// The rows read, the draft first when there is one.
    pub rows: &'a [Row],
    /// The timeline laid out from them.
    pub layout: &'a TimelineLayout,
    /// The merges folded.
    pub folds: Folds<'a>,
    /// Whether the repository has no commit at all.
    pub has_no_commits: bool,
    /// Whether the history has more commits than those read.
    pub has_more: bool,
    /// The commits whose parents were not fetched, where the history of a shallow clone ends; none when it is not
    /// shallow (`graph.shallow-end`).
    pub shallow_ends: &'a [CommitId],
    /// What is being read.
    pub reading: Reading,
    /// The commits loaded when the next page was asked for last, which is not asked for again.
    pub asked_at: Option<usize>,
    /// The row selected.
    pub selected: Option<&'a Selected>,
    /// The commits selected together by `Ctrl`+click or `Shift`+click, the anchor first; empty while one row is
    /// selected alone (`selection.select`).
    pub several: &'a [CommitId],
    /// The row to bring into sight, once for each number.
    pub reveal: Option<(u64, RevealPlace)>,
    /// The history brought out, under a label or of the selected commit.
    pub trace: Option<TraceShown<'a>>,
    /// The label the pointer rests on, while its history is brought out.
    pub hover: Option<&'a HoverTarget>,
    /// When the pointer left the labels, while the history of the last one is still brought out.
    pub hover_left_at: Option<Duration>,
    /// The list of all the labels of a commit, while it is open (`graph.label-overflow`).
    pub label_list: Option<&'a super::label_overflow::LabelList>,
    /// When the list of the labels was closed last, so that it does not open again from where the pointer rested
    /// since before.
    pub label_list_closed_at: Option<Duration>,
    /// The reference of the branch rail the pointer rests on, with since when, whose history is brought out as a
    /// label's is; the window sets it from the rail drawn before the timeline.
    pub rail_hover: Option<(HoverTarget, Duration)>,
    /// The switches of the timeline: the columns among them.
    pub switches: Switches,
    /// The width the graph was dragged to; none while it chooses its own.
    pub graph_width: Option<f32>,
    /// The moment the dates are said from, in seconds since 1970.
    pub now: i64,
    /// The rows shown that a search found, in order, for their marks (`graph.scroll-marks`).
    pub found: &'a [usize],
    /// Whether the rows a search did not find are dimmed (`search.box`).
    pub dims_others: bool,
    /// The draft, when it is the first row.
    pub draft: Option<DraftState>,
    /// What the menus of the commits and of their labels are made from; none while the repository is not read, and
    /// they have only the fold of a merge.
    pub references: Option<ReferenceMenus<'a>>,
    /// The commit box of the Changes page, whose message and **Commit** the draft shows; and whether an operation
    /// runs, which disables them and the entries of its menu.
    pub commit_box: crate::changes::CommitBox<'a>,
    /// What was dropped last on a row, for the menu of the drop while it is open (`drag.drop`).
    pub dropped: Option<&'a crate::drag::Dropped>,
    /// The rows listed before the same history was read again, drawn dimmed under those read until the read reaches
    /// them (D72).
    pub awaited: Option<Awaited<'a>>,
}

/// The rows a timeline listed before the commits of the same history were read again, after a commit, a fetch or
/// **Refresh**: the list keeps their length and the graph their width, and those under the rows read are drawn where
/// they were, dimmed, without labels, taking no click (D72).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Awaited<'a> {
    /// The rows, the draft first when there was one.
    pub rows: &'a [Row],
    /// Their layout, without the labels.
    pub layout: &'a TimelineLayout,
}

impl HistoryState<'_> {
    /// How many rows the list has: those shown, the last that says more are loading, and as many as the rows listed
    /// before while the same history is read again.
    #[must_use]
    pub fn row_count(&self) -> usize {
        let shown = self.layout.len() + usize::from(self.has_more || self.has_shallow_end());
        self.awaited.map_or(shown, |awaited| shown.max(awaited.layout.len()))
    }

    /// Whether the last row says where the history of a shallow clone ends: its last page is read, and it is shallow
    /// (`graph.shallow-end`).
    #[must_use]
    pub fn has_shallow_end(&self) -> bool {
        !self.has_more && !self.shallow_ends.is_empty()
    }

    /// The most lanes a row has, or had before the same history was read again.
    #[must_use]
    pub fn lane_count(&self) -> usize {
        let lanes = self.layout.lane_count;
        self.awaited
            .map_or(lanes, |awaited| lanes.max(awaited.layout.lane_count))
    }
}

/// What the draft of the uncommitted changes shows and offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DraftState {
    /// How many files have a change of any kind, each once.
    pub files: usize,
    /// How many files are staged.
    pub staged: usize,
    /// How many files are changed and not staged, or not tracked.
    pub unstaged: usize,
    /// How many files are in conflict.
    pub conflicted: usize,
    /// Whether there is a commit checked out, to stash onto.
    pub has_commit: bool,
    /// Whether an operation stopped half way, a merge, a rebase, a cherry-pick, a revert or a patch.
    pub is_operation_stopped: bool,
}

/// The merges folded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Folds<'a> {
    /// The merges folded.
    pub folded: &'a HashSet<CommitId>,
    /// Whether every merge is folded, also those loaded later (`graph.fold-all`).
    pub is_all_folded: bool,
}

/// What is being read for the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Reading {
    /// Nothing.
    #[default]
    Nothing,
    /// The next page: a bar runs along the top of the list.
    More,
    /// The pages after the first of a read, as git prints them: no bar, the last row says more are loaded.
    Pages,
    /// The commits from the first page, for another history: the bar runs, and the rows listed are dimmed.
    Again,
}

/// The history brought out in the timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceShown<'a> {
    /// What of each row shown belongs to it.
    pub cells: &'a [CellTrace],
    /// How much the rest fades.
    pub strength: TraceStrength,
}

/// How much what is not in a history brought out fades.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TraceStrength {
    /// Under a label: the rest fades well back (`graph.hover-history`).
    Label,
    /// Of the selected commit: the rest is dimmed less (`graph.trace-selected`).
    Selected,
}
