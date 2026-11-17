//! A comparison beside its diffs (`compare.beside-diff`): a file of the comparison page opens its diff on the left, and
//! the comparison stays on the right as a column, where the inspector is: its two sides with the arrows that swap them,
//! what `compare.summary` says in one line, its commits folded, and its files as a tree, the file shown marked.

use std::collections::BTreeSet;

use model::compare::{CompareSide, CompareView, SummaryInput, summary};
use model::details::TreeNode;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Axis, Button, ButtonKind, Key, Label, List, Modifiers, NamedKey, Rect, Reveal, RevealPlace, Row,
    SelectionMode, Sense, Split, Splitter, Steering, TextKind, Tone, Ui,
};

use super::page::side_icon;
use super::{CompareIntent, CompareRead, CompareState};
use crate::details::{FileLayout, FileRow, FilesOwner, directory_row, file_name, file_start, tree_file_rows};
use crate::diff::{DiffPageState, FileStep};
use crate::graph::{GraphIntent, INSPECTOR_LEAST, INSPECTOR_MOST, INSPECTOR_START, Pane, STEP};
use crate::icons;
use crate::intent::Intent;
use crate::line::INSPECTOR_OVER_BELOW;
use crate::text::compare as words;

/// The most commits the column lists once **Commits** is unfolded; the comparison page lists them all.
const COLUMN_COMMITS: usize = 100;
/// The least width of the diff beside the column.
const DIFF_LEAST: f32 = 240.0;

/// A comparison page with the diff of its files on the left and the comparison as a column on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BesideDiffState<'a> {
    /// The comparison.
    pub compare: CompareState<'a>,
    /// The Diff page of the file shown, or of all the files.
    pub diff: DiffPageState<'a>,
    /// The files of the comparison as a tree.
    pub tree: &'a [TreeNode],
    /// The folders of the tree closed, by their paths.
    pub folded: &'a BTreeSet<String>,
    /// The file the diff shows, by its path, which the column marks; none when it shows several.
    pub marked: Option<&'a str>,
    /// A number that changes each time another file is shown, which brings its row into sight once.
    pub serial: u64,
    /// Whether **Commits** is unfolded in the column.
    pub are_commits_unfolded: bool,
    /// The width of the column as dragged, which is that of the inspector, in whole logical pixels.
    pub column_width: Option<u16>,
}

/// A row of the column, under the sides and what is said of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnRow<'a> {
    /// **Commits**, with how many there are on each side, which folds the commits.
    Commits,
    /// A commit, by its place among the commits.
    Commit(usize),
    /// **N changed files**.
    Files,
    /// A folder or a file of the tree.
    File(FileRow<'a>),
}

/// The comparison page with a file opened, over the rectangle its caller clipped it to: the Diff page on the left, the
/// column of the comparison on the right with the line between them that resizes the column; in the narrow layout the
/// Diff page alone. `Alt+Down` and `Alt+Up` show the next and the previous file, and `Escape` the whole comparison.
///
/// Feature: compare.beside-diff
pub fn beside_diff_view(state: &BesideDiffState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let mut intents = if ui.window().width < INSPECTOR_OVER_BELOW {
        crate::diff::page_view(&state.diff, ui)
    } else {
        split(state, ui, rect)
    };
    let id = state.compare.id;
    for key in ui.unclaimed_keys() {
        let intent = match (key.key, key.modifiers) {
            (Key::Named(NamedKey::ArrowDown), Modifiers::ALT) => CompareIntent::StepFile(id, FileStep::Next),
            (Key::Named(NamedKey::ArrowUp), Modifiers::ALT) => CompareIntent::StepFile(id, FileStep::Previous),
            (Key::Named(NamedKey::Escape), Modifiers::NONE) => CompareIntent::ShowWhole(id),
            _ => continue,
        };
        intents.push(Intent::Compare(intent));
    }
    intents
}

/// The diff and the column side by side, with the line between them that is dragged.
fn split(state: &BesideDiffState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let line = ui.theme().sizes.border;
    let room = (rect.width - line).max(0.0);
    let most = rect.width * INSPECTOR_MOST;
    let width = state
        .column_width
        .map_or(INSPECTOR_START, f32::from)
        .clamp(INSPECTOR_LEAST, most.max(INSPECTOR_LEAST))
        .min(room);
    let splitter = Splitter {
        split: Split::Columns,
        default_size: (room - INSPECTOR_START).max(0.0),
        least_first: DIFF_LEAST.min(room),
        least_second: INSPECTOR_LEAST,
        size: Some(room - width),
        step: STEP,
    };
    let diff = Rect::new(rect.x, rect.y, (room - width).max(0.0), rect.height);
    let column = Rect::new(diff.right() + line, rect.y, width, rect.height);
    let mut intents = ui.clip(diff, |ui| crate::diff::page_view(&state.diff, ui));
    ui.separator(Rect::new(diff.right(), rect.y, line, rect.height), Axis::Vertical);
    ui.region(
        "comparison column",
        (column, AccessRole::Complementary, words::COLUMN),
        |ui| {
            intents.extend(ui.clip(column, |ui| column_view(state, ui, column)));
        },
    );
    let response = ui.splitter_across("comparison column line", rect, &splitter, line);
    if response.is_reset {
        intents.push(Intent::Graph(GraphIntent::SetPane(Pane::Inspector, None)));
    } else if response.is_changed {
        let width = (room - response.size).max(INSPECTOR_LEAST);
        intents.push(Intent::Graph(GraphIntent::SetPane(Pane::Inspector, Some(width))));
    }
    intents
}

/// The column of the comparison: its sides, what is said of them, and the list of its commits and its files.
fn column_view(state: &BesideDiffState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(inset(rect, Insets::all(spacing.medium)), spacing.small);
    intents.extend(sides_row(state, ui, area.top(theme.sizes.control_height)));
    let CompareRead::Read(view) = state.compare.read else {
        let said = if state.compare.read == CompareRead::Failed {
            words::FAILED
        } else {
            words::COMPARING
        };
        let _ = ui.label(area.top(theme.sizes.row_height), &Label::new(said).tone(Tone::Muted));
        return intents;
    };
    let said = summary(&SummaryInput {
        first: &state.compare.target.first,
        second: &state.compare.target.second,
        view,
    });
    let distance = area.top(theme.sizes.row_height);
    let _ = ui.label(distance, &Label::new(&said.distance).kind(TextKind::Detail));
    intents.extend(column_list(state, view, ui, area.rest()));
    intents
}

/// The two sides, each the button that chooses another in its place, and the arrows that swap them between.
fn sides_row(state: &BesideDiffState<'_>, ui: &mut Ui<'_>, row: Rect) -> Vec<Intent> {
    let mut intents = Vec::new();
    let id = state.compare.id;
    let spacing = ui.theme().sizes.spacing;
    let swap = Button::icon(icons::SWAP, words::SWAP_TOOLTIP);
    let (swap_width, _) = ui.button_size(&swap);
    let each = ((row.width - swap_width - 2.0 * spacing.small) * 0.5).max(0.0);
    let mut parts = Area::with_gap(row, spacing.small);
    let target = state.compare.target;
    for (side, is_first) in [(&target.first, true), (&target.second, false)] {
        let cell = parts.left(each);
        if column_side(ui, (side, is_first), cell) {
            intents.push(Intent::Compare(CompareIntent::ChangeSide(id, is_first)));
        }
        if is_first {
            let cell = place(
                parts.left(swap_width),
                (swap_width, row.height),
                (Align::Center, Align::Center),
            );
            if ui.button(("column swap", id.0), cell, &swap).clicked {
                intents.push(Intent::Compare(CompareIntent::Swap(id)));
            }
        }
    }
    intents
}

/// A side in the column, its icon and its name, a button that chooses another in its place; whether it was clicked.
fn column_side(ui: &mut Ui<'_>, (side, is_first): (&CompareSide, bool), cell: Rect) -> bool {
    let tooltip = side.tooltip();
    let button = Button::secondary(&side.name)
        .kind(ButtonKind::Outline)
        .with_icon(side_icon(side))
        .with_tooltip(&tooltip);
    let (width, height) = ui.button_size(&button);
    let placed = place(cell, (width.min(cell.width), height), (Align::Start, Align::Center));
    ui.button(("column side", is_first), placed, &button).clicked
}

/// The rows under the sides: **Commits** and, unfolded, the commits; **N changed files** and the tree of the files.
fn column_rows<'a>(state: &BesideDiffState<'a>, view: &CompareView) -> Vec<ColumnRow<'a>> {
    let mut rows = vec![ColumnRow::Commits];
    if state.are_commits_unfolded {
        rows.extend((0..view.commits.len().min(COLUMN_COMMITS)).map(ColumnRow::Commit));
    }
    rows.push(ColumnRow::Files);
    rows.extend(
        tree_file_rows(state.tree, state.folded)
            .into_iter()
            .map(ColumnRow::File),
    );
    rows
}

/// The list of the commits and the files, the file shown marked and brought into sight once each time it changes.
fn column_list(state: &BesideDiffState<'_>, view: &CompareView, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let rows = column_rows(state, view);
    let height = ui.theme().sizes.row_height + ui.theme().sizes.spacing.small;
    let list = List {
        name: words::COLUMN,
        row_count: rows.len(),
        row_height: height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let marked = |row: &ColumnRow<'_>| match row {
        ColumnRow::File(FileRow::File(index, _)) => view.files.get(*index).map(|file| file.path.as_str()),
        _ => None,
    };
    let reveal = state
        .marked
        .and_then(|path| rows.iter().position(|row| marked(row) == Some(path)))
        .map(|row| Reveal {
            serial: state.serial,
            row,
            place: RevealPlace::Nearest,
        });
    let steering = Steering {
        reveal,
        ..Steering::default()
    };
    let mut intents = Vec::new();
    let _ = ui.steered_list(
        ("comparison column rows", state.compare.id.0),
        (rect, &list),
        &steering,
        |ui, row| {
            let Some(column_row) = rows.get(row.index) else {
                return;
            };
            let is_marked = state.marked.is_some() && marked(column_row) == state.marked;
            intents.extend(column_row_view(state, view, (column_row, is_marked), ui, row));
        },
    );
    intents
}

/// A row of the column, and what a click on it asks.
fn column_row_view(
    state: &BesideDiffState<'_>,
    view: &CompareView,
    (column_row, is_marked): (&ColumnRow<'_>, bool),
    ui: &mut Ui<'_>,
    row: &Row,
) -> Vec<Intent> {
    let id = state.compare.id;
    match column_row {
        ColumnRow::Commits => {
            let target = state.compare.target;
            let said = words::column_commits(
                (&target.first.name, view.only_in_first),
                (&target.second.name, view.only_in_second),
            );
            let arrow = if state.are_commits_unfolded {
                icons::UNFOLDED
            } else {
                icons::FOLDED
            };
            let button = Button::secondary(&said).kind(ButtonKind::Icon).with_icon(arrow);
            let (width, height) = ui.button_size(&button);
            let cell = place(
                row.rect,
                (width.min(row.rect.width), height),
                (Align::Start, Align::Center),
            );
            let clicked = ui.button(("column commits", id.0), cell, &button).clicked;
            clicked
                .then_some(Intent::Compare(CompareIntent::FoldCommits(id)))
                .into_iter()
                .collect()
        }
        ColumnRow::Commit(index) => commit_row(state, view, *index, ui, row).into_iter().collect(),
        ColumnRow::Files => {
            let said = words::changed_files(view.files.len());
            let _ = ui.label(row.rect, &Label::new(&said).kind(TextKind::Strong));
            Vec::new()
        }
        ColumnRow::File(file_row @ FileRow::Directory { .. }) => {
            directory_row(ui, row.rect, (FilesOwner::Comparison(id), file_row))
        }
        ColumnRow::File(FileRow::File(index, depth)) => file_row(state, view, (*index, *depth, is_marked), ui, row)
            .into_iter()
            .collect(),
    }
}

/// A commit only in one side, its short hash and its subject; a click opens its page.
fn commit_row(
    state: &BesideDiffState<'_>,
    view: &CompareView,
    index: usize,
    ui: &mut Ui<'_>,
    row: &Row,
) -> Option<Intent> {
    let commit = view.commits.get(index)?;
    let said = format!("{}  {}", commit.id.short(), commit.subject);
    let button = Button::secondary(&said).kind(ButtonKind::Icon).with_icon(icons::COMMIT);
    let (width, height) = ui.button_size(&button);
    let cell = place(
        row.rect,
        (width.min(row.rect.width), height),
        (Align::Start, Align::Center),
    );
    let id = state.compare.id;
    ui.button(("column commit", index), cell, &button)
        .clicked
        .then(|| Intent::Compare(CompareIntent::OpenCommit(id, commit.id.clone())))
}

/// A file of the tree, marked while its diff is shown; a click shows its diff in place of the one shown.
fn file_row(
    state: &BesideDiffState<'_>,
    view: &CompareView,
    (index, depth, is_marked): (usize, usize, bool),
    ui: &mut Ui<'_>,
    row: &Row,
) -> Option<Intent> {
    let file = view.files.get(index)?;
    let theme = *ui.theme();
    let response = ui.interact(ui.id(("column file", index)), row.rect, Sense::CLICK | Sense::HOVER);
    if is_marked {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let rest = file_start(ui, row.rect, (file, depth));
    file_name(ui, rest, (file, FileLayout::Tree));
    ui.describe(
        response.id,
        row.rect,
        &ui::Description::new(AccessRole::Button, &file.path),
    );
    response
        .clicked
        .then_some(Intent::Compare(CompareIntent::OpenFile(state.compare.id, index)))
}
