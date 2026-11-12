//! A row of the timeline: its cell of the graph, then the message with the labels after it over the author and the
//! date, and the hash at its end; a folded row, a stash, the draft, and the last row while more commits are read.

use std::borrow::Cow;
use std::time::Duration;

use model::commit::Commit;
use model::graph::{Label, LabelKind, Row, ShownRow};
use ui::layout::{Align, Area, place};
use ui::{Badge, BadgeFill, Icon, Point, Rect, Sense, TextKind, TextRun, Tone, Ui};

use super::cell::{CellPlace, Dot, draw_cell};
use super::columns::{ColumnsState, columns_view, hash_room};
use super::draft::{UncommittedRow, uncommitted_row_view};
use super::fold::{FoldsState, fold_merge_view};
use super::intent::{GraphIntent, HoverTarget, HoverWay, Selected};
use super::stashes::{StashRow, stashes_view};
use super::state::HistoryState;
use crate::intent::Intent;
use crate::text::timeline as words;

/// How many labels a commit shows in the narrow layout before it counts the others.
const NARROW_LABELS: usize = 3;
/// How many labels it names then.
const NAMED_WHEN_COUNTED: usize = 2;
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
    /// Whether the window is narrow: the hash is not shown, and a commit with many labels counts them.
    pub is_narrow: bool,
}

/// What the rows drawn in a frame report.
#[derive(Clone, Debug, Default)]
pub(crate) struct RowReport {
    /// The intents of the rows.
    pub intents: Vec<Intent>,
    /// The label under the pointer, with since when it rests there.
    pub label: Option<(HoverTarget, Duration)>,
}

/// Draws a row of the list.
pub(crate) fn draw_row(ui: &mut Ui<'_>, context: &RowContext<'_>, row: &ui::Row, report: &mut RowReport) {
    let layout = context.state.layout;
    let Some(shown) = layout.folded.rows.get(row.index) else {
        loading_row(ui, context, row);
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
    let dot = match (shown, read) {
        (ShownRow::Fold(_), _) | (_, Some(Row::Uncommitted { .. })) => Dot::Dashed,
        (_, Some(Row::Stash { .. })) => Dot::Stash,
        (_, Some(row)) if row.parents().len() > 1 => Dot::Merge,
        _ => Dot::Commit,
    };
    let id = read.and_then(Row::id);
    draw_graph(ui, context, (row.index, graph), (dot, id));
    match (shown, read) {
        (ShownRow::Fold(fold), _) => {
            let title = fold.title(&layout.labels);
            ui.clip(text, |ui| fold_row(ui, text, &title));
        }
        (_, Some(Row::Uncommitted { .. })) => {
            let state = UncommittedRow {
                draft: context.state.draft.unwrap_or_default(),
                is_selected: row.is_selected,
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
    separator(ui, context, row);
    if context.state.dims_others && context.state.found.binary_search(&row.index).is_err() {
        let veil = ui.theme().colors.page_background.with_alpha(DIMMED);
        ui.fill_rect(row.rect, veil);
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
    let trace = context
        .state
        .trace
        .and_then(|trace| trace.cells.get(index).map(|cells| (cells, trace.strength)));
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

/// The color of the line of a row.
fn lane_color(context: &RowContext<'_>, index: usize) -> usize {
    context.state.layout.cells.get(index).map_or(0, |cell| cell.color)
}

/// A commit: the message and the labels over the author and the date, the hash at the end, and the fold arrow of a
/// merge.
fn commit_row(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, rect, commit): (&ui::Row, Rect, &Commit),
    report: &mut RowReport,
) {
    let theme = *ui.theme();
    let switches = context.state.switches;
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

/// The first line of a commit: its message, the fold arrow of a merge and the labels, from the top of the lines.
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
    let color = lane_color(context, row.index);
    let labels = context
        .state
        .layout
        .labels
        .get(&commit.id)
        .map_or(&[][..], Vec::as_slice);
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
        .min(available * SUBJECT_SHARE);
    let gap = theme.sizes.spacing.medium;
    let (pills, pills_width) = fitted_pills(labels, available - kept, context.is_narrow, |pill| {
        ui.badge_width(&pill.badge(color)) + gap
    });
    let subject = TextRun {
        text: &commit.subject,
        style: subject_style,
        color: row.foreground,
        origin: Point::new(lines.x, top),
        max_width: Some((available - pills_width).max(0.0)),
    };
    let mut x = lines.x + ui.text(&subject) + theme.sizes.spacing.medium;
    if folds.is_foldable {
        let cell = Rect::new(x, top, first, first);
        report.intents.extend(ui.clip(cell, |ui| fold_merge_view(&folds, ui)));
        x += arrow;
    }
    for pill in &pills {
        let badge = pill.badge(color);
        let width = ui.badge_width(&badge).min((lines.right() - x).max(0.0));
        // A label with less room than its own height is not drawn at all, rather than squeezed past the end.
        if width < first {
            break;
        }
        let drawn = ui.badge(Rect::new(x, top, width, first), &badge);
        x += width + theme.sizes.spacing.medium;
        hover_label(ui, (drawn, pill), &commit.id, report);
    }
}

/// Draws the labels of a commit as pills from the left of a rectangle, in the color of its line, as its row does: the
/// page of a commit shows them so (`commit-page.view`); a label with less room than its own height is left out, as in
/// a row.
pub(crate) fn draw_labels(ui: &mut Ui<'_>, rect: Rect, (labels, color): (&[Label], usize)) {
    let gap = ui.theme().sizes.spacing.medium;
    let mut x = rect.x;
    for pill in &pills(labels, false) {
        let badge = pill.badge(color);
        let width = ui.badge_width(&badge).min((rect.right() - x).max(0.0));
        if width < rect.height {
            break;
        }
        let _ = ui.badge(Rect::new(x, rect.y, width, rect.height), &badge);
        x += width + gap;
    }
}

/// Notes the label under the pointer, and gives a label its tooltip.
fn hover_label(ui: &mut Ui<'_>, (rect, pill): (Rect, &Pill<'_>), commit: &model::CommitId, report: &mut RowReport) {
    let id = ui.id(("label", commit.as_str(), pill.text.as_ref()));
    let response = ui.interact(id, rect, Sense::HOVER);
    if let Some(tooltip) = &pill.tooltip {
        ui.tooltip(&response, tooltip);
    }
    let Some(since) = ui.hovered_since(&response) else {
        return;
    };
    let way = if ui.modifiers().shift() {
        HoverWay::NotCheckedOut
    } else {
        HoverWay::Reaches
    };
    let target = HoverTarget {
        commit: commit.clone(),
        way,
    };
    report.label = Some((target, since));
}

/// A label as a pill: its words, its icons and how it is filled, or the count of the labels not named.
pub(super) struct Pill<'a> {
    /// Its words.
    pub(super) text: Cow<'a, str>,
    /// Its icons.
    icons: &'static [Icon],
    /// How it is filled.
    fill: BadgeFill,
    /// Its tooltip.
    pub(super) tooltip: Option<String>,
    /// Whether it counts other labels, in the muted tone.
    is_count: bool,
}

impl Pill<'_> {
    /// The pill drawn in the color of the commit's line.
    fn badge(&self, color: usize) -> Badge<'_> {
        Badge {
            text: &self.text,
            icons: self.icons,
            tone: if self.is_count { Tone::Muted } else { Tone::Lane(color) },
            fill: self.fill,
        }
    }
}

/// The share of the first line of a row its message keeps, when it is that long, before the labels take the rest.
const SUBJECT_SHARE: f32 = 0.5;

/// The pills of the labels of a commit that fit in a width, each as wide as `width_of` says with the gap after it, and
/// the width they take: as many named as fit, and the count of the others, `+2`, with their names in its tooltip; none
/// when not even the count fits.
pub(super) fn fitted_pills(
    labels: &[Label],
    room: f32,
    is_narrow: bool,
    width_of: impl Fn(&Pill<'_>) -> f32,
) -> (Vec<Pill<'_>>, f32) {
    for named in (0..=named_labels(labels, is_narrow)).rev() {
        let pills = counted_pills(labels, named);
        let width: f32 = pills.iter().map(&width_of).sum();
        if width <= room {
            return (pills, width);
        }
    }
    (Vec::new(), 0.0)
}

/// How many labels of a commit are named before the count of the others: in the narrow layout, two past three.
fn named_labels(labels: &[Label], is_narrow: bool) -> usize {
    if is_narrow && labels.len() > NARROW_LABELS {
        NAMED_WHEN_COUNTED
    } else {
        labels.len()
    }
}

/// The pills of the labels of a commit: in the narrow layout, two and the count of the others past three.
fn pills(labels: &[Label], is_narrow: bool) -> Vec<Pill<'_>> {
    counted_pills(labels, named_labels(labels, is_narrow))
}

/// The pills of the first labels of a commit, then the count of the others when there are some.
fn counted_pills(labels: &[Label], named: usize) -> Vec<Pill<'_>> {
    let mut pills: Vec<Pill<'_>> = labels.iter().take(named).map(pill).collect();
    if let Some(rest) = labels.get(named..).filter(|rest| !rest.is_empty()) {
        pills.push(Pill {
            text: Cow::Owned(format!("+{}", rest.len())),
            icons: &[],
            fill: BadgeFill::Tint,
            tooltip: Some(
                rest.iter()
                    .map(|label| label.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            is_count: true,
        });
    }
    pills
}

/// The pill of a label.
fn pill(label: &Label) -> Pill<'_> {
    let (icons, tooltip): (&'static [Icon], Option<String>) = match &label.kind {
        LabelKind::Branch {
            worktree: Some(folder), ..
        } => (
            &[Icon::Worktree],
            Some(words::checked_out_in(&folder.display().to_string())),
        ),
        LabelKind::Branch { upstream: Some(_), .. } => (&[Icon::Branch, Icon::Cloud], None),
        LabelKind::Head | LabelKind::Branch { .. } => (&[], None),
        LabelKind::RemoteBranch => (&[Icon::Cloud], None),
        LabelKind::Tag => (&[Icon::Tag], None),
        LabelKind::Stash { .. } => (&[Icon::Stash], None),
    };
    Pill {
        text: Cow::Borrowed(&label.name),
        icons,
        fill: label_fill(label),
        tooltip,
        is_count: false,
    }
}

/// How a label is filled: the one checked out filled with the color of its line, the others outlined.
pub(crate) fn label_fill(label: &Label) -> BadgeFill {
    if label.is_checked_out {
        BadgeFill::Solid
    } else {
        BadgeFill::Outline
    }
}

/// A folded row: the icon of unfolding and what it holds, `3 commits of feature`.
fn fold_row(ui: &mut Ui<'_>, rect: Rect, title: &str) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    ui.icon_in(area.left(theme.sizes.icon), Icon::ChevronRight, Tone::Muted);
    let _ = ui.label(area.rest(), &ui::Label::new(title).tone(Tone::Muted));
    let id = ui.id(("fold row", rect.y.to_bits()));
    let response = ui.interact(id, rect, Sense::HOVER);
    ui.tooltip(&response, words::CLICK_TO_SHOW);
}

/// The last row while more commits are to be read: **Loading more commits…** with a turning icon.
fn loading_row(ui: &mut Ui<'_>, context: &RowContext<'_>, row: &ui::Row) {
    let theme = *ui.theme();
    let rect = Rect::new(
        row.rect.x + context.graph_width + theme.sizes.spacing.large,
        row.rect.y,
        (row.rect.width - context.graph_width - 2.0 * theme.sizes.spacing.large).max(0.0),
        row.rect.height,
    );
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    let icon = area.left(theme.sizes.icon);
    let spinner = place(
        icon,
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Center, Align::Center),
    );
    ui.spinner(spinner);
    let _ = ui.label(area.rest(), &ui::Label::new(words::LOADING_MORE).tone(Tone::Muted));
}

/// The line before a row that starts a day, a week or a month further back, with its name (`graph.date-separators`).
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
    let back = Rect::new(x, row.rect.y - height * 0.5, width, height);
    ui.fill_rect(back, theme.colors.page_background);
    let run = TextRun {
        text: name,
        style,
        color: theme.colors.muted_foreground,
        origin: Point::new(x + theme.sizes.spacing.small, back.y),
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
