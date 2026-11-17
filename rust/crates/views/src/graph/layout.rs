//! What the timeline is drawn from besides the rows read: the rows shown with the merges folded, the lines of the
//! graph, the labels, the commits not pushed, the lines between days and months and the branch each line comes from.
//! It is made again when the rows, the folds, the repository or the settings change, not for each frame; a page added
//! under the rows is laid out alone, from where the layout stopped (D54).

use std::collections::{BTreeMap, HashMap, HashSet};

use model::CommitId;
use model::commit::Commit;
use model::graph::{
    CellTrace, DateTime, FoldedRows, GraphCell, GraphInput, GraphSettings, Label, LaneState, Row, ShownRow, Trace,
    date_separators, fold_rows, foldable_merges, labels, lane_tips_after, lanes_after, one_lane, one_lane_after,
    trace_history, unpushed,
};
use model::state::RepositoryState;

/// What the layout of the timeline is made from.
#[derive(Clone, Copy, Debug)]
pub struct LayoutInput<'a> {
    /// The rows read, the draft first when there is one.
    pub rows: &'a [Row],
    /// The repository, as last read: its references and what is checked out.
    pub repository: &'a RepositoryState,
    /// The merges folded.
    pub folded: &'a HashSet<CommitId>,
    /// `graph.combineUpstream`.
    pub combine_upstream: bool,
    /// The time the lines between days and months are counted from; none while `graph.dateSeparators` is off.
    pub separators_at: Option<DateTime>,
    /// `graph.oneLane`: every commit on one line, without lanes (`graph.one-lane`).
    pub one_lane: bool,
}

/// The timeline laid out: everything about its rows that does not change from one frame to the next.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct TimelineLayout {
    /// The rows shown, with the merges folded.
    pub folded: FoldedRows,
    /// The cell of the graph of each row shown.
    pub cells: Vec<GraphCell>,
    /// The most lanes a row has.
    pub lane_count: usize,
    /// The labels of each commit.
    pub labels: HashMap<CommitId, Vec<Label>>,
    /// The commits not pushed, with the upstream each would go to.
    pub unpushed: HashMap<CommitId, String>,
    /// The rows shown that start a day, a week or a month further back, with its name.
    pub separators: BTreeMap<usize, String>,
    /// For each row shown, and each lane a line comes down into it in, the row shown of the tip of that line.
    pub tips: Vec<Vec<Option<usize>>>,
    /// The row shown of each commit shown, a folded row by the first commit it holds.
    pub shown_of: HashMap<CommitId, usize>,
    /// For each row shown, the row read it stands for: a folded row, the last commit it holds.
    pub read_of: Vec<usize>,
    /// The merges that can be folded.
    pub foldable: HashSet<CommitId>,
    /// The row shown of the commit checked out, when it is shown.
    pub checked_out: Option<usize>,
    /// The color of the line of each local branch whose commit is shown, by the branch's name, so that its dot in the
    /// branch rail matches its labels in the graph.
    pub branch_colors: HashMap<String, usize>,
    /// The same for each remote branch whose commit is shown, by its name, as `origin/main`, for its ring.
    pub remote_colors: HashMap<String, usize>,
    /// Where the layout stopped, for a page added under its rows to be laid out alone; none while merges are folded.
    pub end: Option<LayoutEnd>,
}

/// Where a layout stopped: what the rows of a page added under its last row are laid out from.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LayoutEnd {
    /// How many rows read are laid out.
    pub rows: usize,
    /// The lanes under the last row.
    pub lanes: LaneState,
    /// The tip of each lane at the bottom edge of the last row.
    pub tips: Vec<Option<usize>>,
    /// `graph.combineUpstream`, as the rows were laid out.
    pub combine_upstream: bool,
    /// The time the lines between days and months were counted from.
    pub separators_at: Option<DateTime>,
    /// `graph.oneLane`, as the rows were laid out.
    pub one_lane: bool,
}

/// The color of the line of each local branch and each remote branch whose commit is shown, by its name.
fn reference_colors(
    repository: &RepositoryState,
    cells: &[GraphCell],
    shown_of: &HashMap<CommitId, usize>,
) -> (HashMap<String, usize>, HashMap<String, usize>) {
    let color_of = |commit: &CommitId| cells.get(*shown_of.get(commit)?).map(|cell| cell.color);
    let branch_colors = repository
        .branches
        .iter()
        .filter_map(|branch| Some((branch.name.clone(), color_of(&branch.commit)?)))
        .collect();
    let remote_colors = repository
        .remote_branches
        .iter()
        .filter_map(|branch| Some((branch.name.clone(), color_of(&branch.commit)?)))
        .collect();
    (branch_colors, remote_colors)
}

impl TimelineLayout {
    /// The timeline laid out from its rows and the repository.
    #[must_use]
    pub fn new(input: &LayoutInput<'_>) -> TimelineLayout {
        let folded = fold_rows(input.rows, input.folded);
        let inputs = folded.graph_inputs(input.rows);
        let (cells, lanes) = if input.one_lane {
            (one_lane(&inputs), LaneState::default())
        } else {
            lanes_after(&LaneState::default(), &inputs)
        };
        let lane_count = cells.iter().map(GraphCell::lane_count).max().unwrap_or(0);
        let labels = labels_of(input);
        let named: Vec<bool> = inputs
            .iter()
            .zip(&folded.rows)
            .map(|(row, shown)| {
                matches!(shown, ShownRow::Row { .. }) && row.id.is_some_and(|id| labels.contains_key(id))
            })
            .collect();
        let (tips, bottom) = lane_tips_after(Vec::new(), (&cells, &named), 0);
        let end = input.folded.is_empty().then_some(LayoutEnd {
            rows: input.rows.len(),
            lanes,
            tips: bottom,
            combine_upstream: input.combine_upstream,
            separators_at: input.separators_at,
            one_lane: input.one_lane,
        });
        let separators = input
            .separators_at
            .map_or_default(|now| date_separators(&shown_dates(input.rows, &folded), now));
        let shown_of = inputs
            .iter()
            .enumerate()
            .filter_map(|(index, row)| row.id.map(|id| (id.clone(), index)))
            .collect::<HashMap<_, _>>();
        let checked_out = input
            .repository
            .head
            .commit
            .as_ref()
            .and_then(|commit| shown_of.get(commit).copied());
        let (branch_colors, remote_colors) = reference_colors(input.repository, &cells, &shown_of);
        TimelineLayout {
            read_of: read_rows(input.rows, &folded),
            unpushed: unpushed(
                input.rows,
                &input.repository.branches,
                &input.repository.remote_branches,
            ),
            foldable: foldable_merges(input.rows),
            folded,
            cells,
            lane_count,
            labels,
            separators,
            tips,
            shown_of,
            checked_out,
            branch_colors,
            remote_colors,
            end,
        }
    }

    /// Lays out the rows of a page added under those of this layout alone, from where it stopped, as
    /// [`TimelineLayout::new`] would lay out all of them. Changes nothing, and says so, when it cannot: merges are
    /// folded, there are fewer rows than it laid out, or the settings it was laid out with changed.
    pub fn extend(&mut self, input: &LayoutInput<'_>) -> bool {
        let Some(end) = self.end.take_if(|end| {
            input.folded.is_empty()
                && end.rows <= input.rows.len()
                && end.combine_upstream == input.combine_upstream
                && end.separators_at == input.separators_at
                && end.one_lane == input.one_lane
        }) else {
            return false;
        };
        let first = end.rows;
        let inputs: Vec<GraphInput<'_>> = input
            .rows
            .get(first..)
            .unwrap_or_default()
            .iter()
            .map(Row::graph_input)
            .collect();
        self.labels = labels_of(input);
        let (cells, lanes, above_tips) = if input.one_lane {
            let cells = self.one_lane_after(input.rows, first, &inputs);
            (cells, LaneState::default(), self.bottom_tips_of_last(input.rows, first))
        } else {
            let (cells, lanes) = lanes_after(&end.lanes, &inputs);
            (cells, lanes, end.tips)
        };
        let named: Vec<bool> = inputs
            .iter()
            .map(|row| row.id.is_some_and(|id| self.labels.contains_key(id)))
            .collect();
        let (tips, bottom) = lane_tips_after(above_tips, (&cells, &named), first);
        if let Some(now) = input.separators_at {
            self.separators
                .extend(separators_after(input.rows, first, &self.separators, now));
        }
        self.lane_count = cells
            .iter()
            .map(GraphCell::lane_count)
            .fold(self.lane_count, usize::max);
        for (index, row) in (first..).zip(&inputs) {
            if let Some(id) = row.id {
                let _ = self.shown_of.insert(id.clone(), index);
            }
            self.folded.rows.push(ShownRow::Row { index, parents: None });
            self.read_of.push(index);
        }
        self.cells.extend(cells);
        self.tips.extend(tips);
        self.rework_whole(input);
        self.end = Some(LayoutEnd {
            rows: input.rows.len(),
            lanes,
            tips: bottom,
            ..end
        });
        true
    }

    /// Lays out on one line the rows of a page added under the `first` rows laid out, giving the last of those its line
    /// down to them, which it did not have while no row was under it (`graph.one-lane`).
    fn one_lane_after(&mut self, rows: &[Row], first: usize, inputs: &[GraphInput<'_>]) -> Vec<GraphCell> {
        let is_dashed = first
            .checked_sub(1)
            .and_then(|last| rows.get(last))
            .is_some_and(Row::is_dashed);
        let above = first.checked_sub(1).and_then(|last| self.cells.get_mut(last));
        one_lane_after(above.map(|cell| (cell, is_dashed)), inputs)
    }

    /// The tip of the one lane at the bottom edge of the last of the `first` rows laid out on one line, given its line
    /// down: its tip at the top edge, which it passes on, or itself when it is named or nothing comes down to it.
    fn bottom_tips_of_last(&self, rows: &[Row], first: usize) -> Vec<Option<usize>> {
        let Some(last) = first.checked_sub(1) else {
            return Vec::new();
        };
        let (Some(cell), Some(top)) = (self.cells.get(last), self.tips.get(last)) else {
            return Vec::new();
        };
        let is_named = rows
            .get(last)
            .and_then(Row::id)
            .is_some_and(|id| self.labels.contains_key(id));
        lane_tips_after(top.clone(), (std::slice::from_ref(cell), &[is_named]), last).1
    }

    /// Works out again what depends on every row, after rows were added: the row of the commit checked out, the color
    /// of each branch, the commits not pushed and the merges that can be folded.
    fn rework_whole(&mut self, input: &LayoutInput<'_>) {
        self.checked_out = input
            .repository
            .head
            .commit
            .as_ref()
            .and_then(|commit| self.shown_of.get(commit).copied());
        (self.branch_colors, self.remote_colors) = reference_colors(input.repository, &self.cells, &self.shown_of);
        self.unpushed = unpushed(
            input.rows,
            &input.repository.branches,
            &input.repository.remote_branches,
        );
        self.foldable = foldable_merges(input.rows);
    }

    /// How many rows are shown.
    #[must_use]
    pub fn len(&self) -> usize {
        self.folded.rows.len()
    }

    /// Whether no row is shown.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folded.rows.is_empty()
    }

    /// The hash of the commit of a row shown, among the rows this layout was made of; a folded row by the first
    /// commit it holds, none for the draft.
    #[must_use]
    pub fn id_at<'a>(&'a self, rows: &'a [Row], shown: usize) -> Option<&'a CommitId> {
        match self.folded.rows.get(shown)? {
            ShownRow::Row { index, .. } => rows.get(*index)?.id(),
            ShownRow::Fold(fold) => Some(&fold.id),
        }
    }

    /// For each row shown, whether its dot and its lines belong to a history, from the rows this layout was made of;
    /// none when the commit it is about is not shown.
    #[must_use]
    pub fn trace(&self, rows: &[Row], wanted: &Trace) -> Option<Vec<CellTrace>> {
        let about = match wanted {
            Trace::Reaches(commit) | Trace::Around(commit) | Trace::HasNot { commit, .. } => commit,
        };
        if !self.shown_of.contains_key(about) {
            return None;
        }
        let traced: Vec<Row> = self.folded.graph_inputs(rows).iter().map(trace_row).collect();
        Some(trace_history(&traced, &self.cells, wanted))
    }
}

/// The labels of the commits, from the references of the repository.
fn labels_of(input: &LayoutInput<'_>) -> HashMap<CommitId, Vec<Label>> {
    labels(
        input.repository,
        &GraphSettings {
            combine_upstream: input.combine_upstream,
        },
    )
}

/// The lines between days and months among rows added under others, none folded: the period furthest back above them
/// is the one of the row of the last line, or else of the first row with a date.
fn separators_after(
    rows: &[Row],
    first: usize,
    separators: &BTreeMap<usize, String>,
    now: DateTime,
) -> BTreeMap<usize, String> {
    let date = |row: &Row| row.commit().map(|commit| commit.author_date.0);
    let furthest = separators
        .keys()
        .next_back()
        .and_then(|index| rows.get(*index))
        .and_then(date)
        .or_else(|| rows.get(..first)?.iter().find_map(date));
    let mut dates = vec![furthest];
    dates.extend(rows.get(first..).unwrap_or_default().iter().map(date));
    date_separators(&dates, now)
        .into_iter()
        .map(|(index, name)| ((first + index).saturating_sub(1), name))
        .collect()
}

/// The date of each row shown: a commit's or a stash's, none for the draft and a folded row.
fn shown_dates(rows: &[Row], folded: &FoldedRows) -> Vec<Option<i64>> {
    folded
        .rows
        .iter()
        .map(|shown| match shown {
            ShownRow::Row { index, .. } => rows
                .get(*index)
                .and_then(Row::commit)
                .map(|commit| commit.author_date.0),
            ShownRow::Fold(_) => None,
        })
        .collect()
}

/// The row read each row shown stands for: a folded row, the last commit it holds, which is under the others.
fn read_rows(rows: &[Row], folded: &FoldedRows) -> Vec<usize> {
    let mut last = 0;
    let index_of: HashMap<&CommitId, usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.id().map(|id| (id, index)))
        .collect();
    folded
        .rows
        .iter()
        .map(|shown| {
            last = match shown {
                ShownRow::Row { index, .. } => *index,
                ShownRow::Fold(fold) => fold
                    .commits
                    .iter()
                    .filter_map(|commit| index_of.get(commit).copied())
                    .max()
                    .unwrap_or(last),
            };
            last
        })
        .collect()
}

/// A row as the history traces see it: its hash and the parents its lines go to, nothing else.
fn trace_row(input: &GraphInput<'_>) -> Row {
    match input.id {
        Some(id) => Row::Commit(Commit {
            id: id.clone(),
            parents: input.parents.to_vec(),
            author_name: String::new(),
            author_email: String::new(),
            author_date: model::Timestamp(0),
            subject: String::new(),
        }),
        None => Row::Uncommitted {
            parent: input.parents.first().cloned(),
            files: 0,
        },
    }
}
