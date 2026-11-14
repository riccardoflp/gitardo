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
