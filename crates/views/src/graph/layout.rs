//! What the timeline is drawn from besides the rows read: the rows shown with the merges folded, the lines of the
//! graph, the labels, the commits not pushed, the lines between days and months and the branch each line comes from.
//! It is made again when the rows, the folds, the repository or the settings change, not for each frame.

use std::collections::{BTreeMap, HashMap, HashSet};

use model::CommitId;
use model::commit::Commit;
use model::graph::{
    CellTrace, DateTime, FoldedRows, GraphCell, GraphInput, GraphSettings, Label, Row, ShownRow, Trace,
    date_separators, fold_rows, foldable_merges, labels, lane_tips, lanes, trace_history, unpushed,
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
}

impl TimelineLayout {
    /// The timeline laid out from its rows and the repository.
    #[must_use]
    pub fn new(input: &LayoutInput<'_>) -> TimelineLayout {
        let folded = fold_rows(input.rows, input.folded);
        let inputs = folded.graph_inputs(input.rows);
        let cells = lanes(&inputs);
        let lane_count = cells.iter().map(GraphCell::lane_count).max().unwrap_or(0);
        let labels = labels(
            input.repository,
            &GraphSettings {
                combine_upstream: input.combine_upstream,
            },
        );
        let named: Vec<bool> = inputs
            .iter()
            .zip(&folded.rows)
            .map(|(row, shown)| {
                matches!(shown, ShownRow::Row { .. }) && row.id.is_some_and(|id| labels.contains_key(id))
            })
            .collect();
        let tips = lane_tips(&cells, &named);
        let separators = input
            .separators_at
            .map(|now| date_separators(&shown_dates(input.rows, &folded), now))
            .unwrap_or_default();
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
        }
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
