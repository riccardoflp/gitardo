//! A row of the timeline: its cell of the graph, then the labels and the message over the author and the date, and the
//! hash at its end; a folded row, a stash, the draft, and the last row while more commits are read.

use std::time::Duration;

use model::commit::Commit;
use model::graph::{CellTrace, Row, ShownRow};
use ui::layout::{Align, Area, place};
use ui::{Point, Rect, Sense, TextKind, TextRun, Ui};

use super::cell::{CellPlace, Dot, draw_cell};
use super::columns::{ColumnsState, columns_view, hash_room};
use super::commit_menu::PointedLabel;
use super::compact::{CompactRow, CompactWidths, RowLines, compact_rows_view};
use super::draft::{UncommittedRow, uncommitted_row_view};
use super::fold::{FoldsState, fold_merge_view};
use super::intent::{GraphIntent, HoverTarget, Selected};
use super::label_overflow::LabelList;
use super::labels::{MESSAGE_SHARE, RowLabels, row_labels};
use super::stashes::{StashRow, stashes_view};
use super::{state::HistoryState, state::TraceStrength};
use crate::intent::Intent;
use crate::text::timeline as words;

/// How opaque the veil over a row a search did not find is, out of 255.
const DIMMED: u8 = 150;

/// Where the rows are drawn, the same for each.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RowContext<'a> {
    /// The timeline.
    pub state: &'a HistoryState<'a>,
    /// The width of the graph.
    pub graph_width: f32,
    /// The width of each lane.
    pub lane_width: f32,
    /// Whether the window is narrow: the hash is not shown.
    pub is_narrow: bool,
    /// The widths of the columns at the end of compact rows; none while each row has two lines
    /// (`graph.compact-rows`).
    pub compact: Option<CompactWidths>,
}

/// What the rows drawn in a frame report.
#[derive(Clone, Debug, Default)]
pub(crate) struct RowReport {
    /// The intents of the rows.
    pub intents: Vec<Intent>,
    /// The label under the pointer, with since when it rests there.
    pub label: Option<(HoverTarget, Duration)>,
    /// The label the pointer is on, for its menu and its double click.
    pub pointed: Option<PointedLabel>,
    /// The list of the labels of a commit the pointer asks for (`graph.label-overflow`).
    pub open_labels: Option<LabelList>,
    /// While the list is open and its row drawn: the row, the label it opened from and all the labels of the row.
    pub open_anchor: Option<(usize, Rect, Rect)>,
}

/// The dot of a row: a ring in dashes for the draft and a fold, a square for a stash, a ring for a merge, and a filled
/// dot for any other commit.
pub(super) fn dot_of(shown: &ShownRow, read: Option<&Row>) -> Dot {
    match (shown, read) {
        (ShownRow::Fold(_), _) | (_, Some(Row::Uncommitted { .. })) => Dot::Dashed,
        (_, Some(Row::Stash { .. })) => Dot::Stash,
        (_, Some(row)) if row.parents().len() > 1 => Dot::Merge,
        _ => Dot::Commit,
    }
}

/// Draws a row of the list.
pub(crate) fn draw_row(ui: &mut Ui<'_>, context: &RowContext<'_>, row: &ui::Row, report: &mut RowReport) {
    let layout = context.state.layout;
    let Some(shown) = layout.folded.rows.get(row.index) else {
        super::load_more::loading_row(ui, context.graph_width, row);
        return;
    };
    let graph = Rect::new(row.rect.x, row.rect.y, context.graph_width, row.rect.height);
    let spacing = ui.theme().sizes.spacing;
    let text = Rect::new(
        graph.right() + spacing.large,
        row.rect.y,
        (row.rect.right() - graph.right() - 2.0 * spacing.large).max(0.0),
        row.rect.height,
    );
    let read = layout
        .read_of
        .get(row.index)
        .and_then(|index| context.state.rows.get(*index));
    let dot = dot_of(shown, read);
    let id = read.and_then(Row::id);
    draw_graph(ui, context, (row.index, graph), (dot, id));
    let opacity = super::cell::row_opacity(trace_of(context, row.index));
    ui.faded(opacity, |ui| row_text(ui, context, (row, text, shown, read), report));
    separator(ui, context, row);
    if context.state.dims_others && context.state.found.binary_search(&row.index).is_err() {
        let veil = ui.theme().colors.page_background.with_alpha(DIMMED);
        ui.fill_rect(row.rect, veil);
    }
}

/// What a row shows right of its cell of the graph: its message, labels, author, date and hash, or what a fold, the
/// draft or a stash shows.
fn row_text(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, text, shown, read): (&ui::Row, Rect, &ShownRow, Option<&Row>),
    report: &mut RowReport,
) {
    let layout = context.state.layout;
    match (shown, read) {
        (ShownRow::Fold(fold), _) => {
            let title = fold.title(&layout.labels);
            ui.clip(text, |ui| super::fold::fold_row(ui, text, &title));
        }
        (_, Some(Row::Uncommitted { .. })) => {
            let state = UncommittedRow {
                draft: context.state.draft.unwrap_or_default(),
                is_selected: row.is_selected,
                commit: context.state.commit_box,
                lines: RowLines::of(context.state.switches),
            };
            report
                .intents
                .extend(ui.clip(text, |ui| uncommitted_row_view(&state, ui)));
        }
        (_, Some(Row::Stash { commit, index })) => {
            let state = StashRow {
                commit,
                index: *index,
                color: lane_color(context, row.index),
            };
            report.intents.extend(ui.clip(text, |ui| stashes_view(&state, ui)));
        }
        (_, Some(Row::Commit(commit))) => commit_row(ui, context, (row, text, commit), report),
        (_, None) => {}
    }
}

/// Draws the cell of the graph of a row, faded by the history brought out.
fn draw_graph(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (index, rect): (usize, Rect),
    (dot, id): (Dot, Option<&model::CommitId>),
) {
    let layout = context.state.layout;
    let Some(cell) = layout.cells.get(index) else {
        return;
    };
    let place = CellPlace {
        rect,
        lane_width: context.lane_width,
        dot,
        is_unpushed: id.is_some_and(|id| layout.unpushed.contains_key(id)),
    };
    let trace = trace_of(context, index);
    ui.clip(rect, |ui| draw_cell(ui, cell, &place, trace));
    if let Some(upstream) = id.and_then(|id| layout.unpushed.get(id)) {
        let center = Point::new(
            super::cell::lane_x(rect, context.lane_width, cell.lane),
            rect.y + rect.height * 0.5,
        );
        let side = ui.theme().sizes.graph_dot * 3.0;
        let dot_rect = Rect::new(center.x - side * 0.5, center.y - side * 0.5, side, side);
        let key = ("unpushed", index);
        let id = ui.id(key);
        let response = ui.interact(id, dot_rect, Sense::HOVER);
        ui.tooltip(&response, &words::not_pushed_to(upstream));
    }
}

/// What the history brought out says of a row, and how strongly it is brought out; none while nothing is.
fn trace_of<'a>(context: &RowContext<'a>, index: usize) -> Option<(&'a CellTrace, TraceStrength)> {
    let trace = context.state.trace?;
    trace.cells.get(index).map(|cells| (cells, trace.strength))
}

/// The color of the line of a row.
fn lane_color(context: &RowContext<'_>, index: usize) -> usize {
    context.state.layout.cells.get(index).map_or(0, |cell| cell.color)
}

/// A commit: the labels and the message over the author and the date, the hash at the end, a merge's fold arrow.
fn commit_row(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, rect, commit): (&ui::Row, Rect, &Commit),
    report: &mut RowReport,
) {
    let theme = *ui.theme();
    let switches = context.state.switches;
    if let Some(widths) = context.compact {
        compact_row(ui, context, (row, rect, commit), widths, report);
        return;
    }
    let hash = hash_room(ui, switches, context.is_narrow);
    let mut area = Area::with_gap(rect, theme.sizes.spacing.large);
    let _hash = area.right(hash);
    let lines = area.rest();
    let subject_style = ui.text_style(if row.is_selected {
        TextKind::Strong
    } else {
        TextKind::Medium
    });
    let first = ui.line_height(subject_style);
    let second = ui.line_height(ui.text_style(TextKind::Detail));
    let gap = theme.sizes.spacing.tiny + 1.0;
    let top = place(
        lines,
        (lines.width, first + gap + second),
        (Align::Start, Align::Center),
    )
    .y;
    subject_line(ui, context, (row, lines, top, commit), report);
    let columns = ColumnsState {
        author: &commit.author_name,
        date: commit.author_date.0,
        hash: commit.id.short(),
        switches,
        now: context.state.now,
        is_narrow: context.is_narrow,
        top: top + first + gap,
    };
    report.intents.extend(ui.clip(rect, |ui| columns_view(&columns, ui)));
}

/// A commit on one line (`graph.compact-rows`): its labels and its message, then at the right end its author, its date
/// and its hash, each in its column.
fn compact_row(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, rect, commit): (&ui::Row, Rect, &Commit),
    widths: CompactWidths,
    report: &mut RowReport,
) {
    let room = widths.room(ui.theme().sizes.spacing.large);
    let line = Rect::new(rect.x, rect.y, (rect.width - room).max(0.0), rect.height);
    let style = ui.text_style(if row.is_selected {
        TextKind::Strong
    } else {
        TextKind::Medium
    });
    let top = place(line, (line.width, ui.line_height(style)), (Align::Start, Align::Center)).y;
    subject_line(ui, context, (row, line, top, commit), report);
    let state = CompactRow {
        columns: ColumnsState {
            author: &commit.author_name,
            date: commit.author_date.0,
            hash: commit.id.short(),
            switches: context.state.switches,
            now: context.state.now,
            is_narrow: context.is_narrow,
            top,
        },
        widths,
    };
    report.intents.extend(ui.clip(rect, |ui| compact_rows_view(&state, ui)));
}

/// The first line of a commit, from the top of the lines: its labels from the start, then its message, then the fold
/// arrow of a merge.
fn subject_line(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, lines, top, commit): (&ui::Row, Rect, f32, &Commit),
    report: &mut RowReport,
) {
    let theme = *ui.theme();
    let subject_style = ui.text_style(if row.is_selected {
        TextKind::Strong
    } else {
        TextKind::Medium
    });
    let first = ui.line_height(subject_style);
    let folds = FoldsState {
        merge: &commit.id,
        is_foldable: context.state.layout.foldable.contains(&commit.id),
        is_folded: context.state.folds.folded.contains(&commit.id),
        is_shown: row.is_hovered || row.is_selected,
    };
    let arrow = if folds.is_foldable {
        first + theme.sizes.spacing.medium
    } else {
        0.0
    };
    let available = (lines.width - arrow).max(0.0);
    let kept = ui
        .measure(&commit.subject, subject_style)
        .min(available * MESSAGE_SHARE);
    let labels = RowLabels {
        shown: row.index,
        commit: &commit.id,
        color: lane_color(context, row.index),
    };
    let room = Rect::new(lines.x, top, (available - kept).max(0.0), first);
    let start = row_labels(ui, context, labels, room, report);
    let subject = TextRun {
        text: &commit.subject,
        style: subject_style,
        color: row.foreground,
        origin: Point::new(start, top),
        max_width: Some((lines.x + available - start).max(0.0)),
    };
    let x = start + ui.text(&subject) + theme.sizes.spacing.medium;
    if folds.is_foldable {
        let cell = Rect::new(x, top, first, first);
        report.intents.extend(ui.clip(cell, |ui| fold_merge_view(&folds, ui)));
    }
}

/// The line before a row that starts a day, a week or a month further back, with its name (`graph.date-separators`):
/// at the start of the line, or between compact rows where the messages end, before the columns: their labels leave
/// it no room at the start, and their columns none at the end.
fn separator(ui: &mut Ui<'_>, context: &RowContext<'_>, row: &ui::Row) {
    let Some(name) = context.state.layout.separators.get(&row.index) else {
        return;
    };
    let theme = *ui.theme();
    let x = row.rect.x + context.graph_width + theme.sizes.spacing.large;
    let line = Rect::new(x, row.rect.y, (row.rect.right() - x).max(0.0), theme.sizes.border);
    ui.fill_rect(line, theme.colors.border);
    let style = ui.text_style(TextKind::Small);
    let width = ui.measure(name, style) + 2.0 * theme.sizes.spacing.small;
    let height = ui.line_height(style);
    let back = if let Some(widths) = context.compact {
        let between = row.rect.height - ui.line_height(ui.text_style(TextKind::Detail));
        let covered = height.min(between);
        let end = row.rect.right() - theme.sizes.spacing.large - widths.room(theme.sizes.spacing.large);
        let left = (end - width).max(x);
        Rect::new(left, row.rect.y - covered * 0.5, width, covered)
    } else {
        Rect::new(x, row.rect.y - height * 0.5, width, height)
    };
    ui.fill_rect(back, theme.colors.page_background);
    let run = TextRun {
        text: name,
        style,
        color: theme.colors.muted_foreground,
        origin: Point::new(back.x + theme.sizes.spacing.small, row.rect.y - height * 0.5),
        max_width: None,
    };
    let _ = ui.text(&run);
}

/// The row selected, as the list counts its rows.
pub(crate) fn selected_row(state: &HistoryState<'_>) -> Option<usize> {
    match state.selected? {
        Selected::Draft => state.draft.is_some().then_some(0),
        Selected::Commit(commit) => state.layout.shown_of.get(commit).copied().filter(|index| {
            // A commit put away under a fold is not selected: the fold row holds it under the first one's hash.
            !matches!(state.layout.folded.rows.get(*index), Some(ShownRow::Fold(_)))
        }),
    }
}

/// What selecting a row asks for: a folded row unfolds, the row after the last commit selects nothing.
pub(crate) fn select_row(state: &HistoryState<'_>, index: usize) -> Option<GraphIntent> {
    match state.layout.folded.rows.get(index)? {
        ShownRow::Fold(fold) => Some(GraphIntent::Unfold(fold.merge.clone())),
        ShownRow::Row { index: read, .. } => match state.rows.get(*read)? {
            Row::Uncommitted { .. } => Some(GraphIntent::Select(Selected::Draft)),
            row => row.id().map(|id| GraphIntent::Select(Selected::Commit(id.clone()))),
        },
    }
}
