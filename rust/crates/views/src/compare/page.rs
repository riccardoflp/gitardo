//! The comparison page: the two sides, what `compare.summary` says of them, the box that narrows it, the mode, the
//! commits only in each side and the files that differ.

use model::compare::{CompareSide, Diverged, FileList, Side, SummaryInput, summary};
use model::state::ChangeKind;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Label, MenuEntry, MenuItem, Rect, TextKind, Tone, Ui};

use super::parts::{commits, commits_height, narrow_to_path, set_mode, swap};
use super::{CompareIntent, CompareRead, CompareState};
use crate::icons;
use crate::intent::Intent;
use crate::text::compare as words;

/// The widest the column of the page is.
const COLUMN_WIDTH: f32 = 960.0;

/// The comparison page, over the rectangle its caller clipped it to: **Comparison** and the two sides, each a button
/// that chooses another in its place, its short hash that opens its commit, **gone** when its name no longer stands for
/// a commit, and the arrows that swap them between; what `compare.summary` says; the box that narrows it; the mode,
/// **Open in the Diff page** and **Copy as patch**; the commits only in each side; and the files that differ, a click
/// opening one in the Diff page and the arrow of each taking the version of the second side.
///
/// Feature: compare.page
pub fn page(state: &CompareState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = theme.sizes.row_height;
    let control = theme.sizes.control_height;
    let width = COLUMN_WIDTH.min((rect.width - 2.0 * spacing.huge).max(0.0));
    let column_area = Rect::new(rect.x + (rect.width - width) * 0.5, rect.y, width, rect.height);
    let height = content_height(state, (line, control, spacing.large));
    ui.region("comparison", (rect, AccessRole::Main, words::PAGE), |ui| {
        let _ = ui.scroll_area(
            ("comparison page", state.id.0),
            column_area,
            (width, height),
            |ui, content| {
                let mut column = Area::with_gap(inset(content, Insets::symmetric(0.0, spacing.large)), spacing.medium);
                let _ = ui.heading(column.top(line * 1.5), words::PAGE);
                sides(state, ui, column.top(control), &mut intents);
                let CompareRead::Read(view) = state.read else {
                    let said = if state.read == CompareRead::Failed {
                        words::FAILED
                    } else {
                        words::COMPARING
                    };
                    let _ = ui.label(column.top(line), &Label::new(said).tone(Tone::Muted));
                    return;
                };
                said(state, view, ui, &mut column);
                intents.extend(ui.clip(column.top(control), |ui| narrow_to_path(state, ui)));
                intents.extend(ui.clip(column.top(control), |ui| set_mode(state, ui)));
                let commit_rows = commits_height(state, line);
                if commit_rows > 0.0 {
                    intents.extend(ui.clip(column.top(commit_rows), |ui| commits(state, ui)));
                }
                let _ = ui.label(
                    column.top(line),
                    &Label::new(&words::changed_files(view.files.len())).kind(TextKind::Strong),
                );
                for (index, file) in view.files.iter().enumerate() {
                    file_row(state, (index, file), ui, column.top(line), &mut intents);
                }
            },
        );
    });
    intents
}

/// What `compare.summary` says of the two sides: how far apart, where they diverged, and what merging would do.
fn said(state: &CompareState<'_>, view: &model::compare::CompareView, ui: &mut Ui<'_>, column: &mut Area) {
    let line = ui.theme().sizes.row_height;
    let said = summary(&SummaryInput {
        first: &state.target.first,
        second: &state.target.second,
        view,
    });
    let _ = ui.label(column.top(line), &Label::new(&said.distance));
    if let Some(diverged) = &said.diverged {
        let text = match diverged {
            Diverged::At { text, .. } => text.as_str(),
            Diverged::NeverMeet => "Their histories never meet.",
        };
        let _ = ui.label(column.top(line), &Label::new(text).tone(Tone::Muted));
    }
    if let Some(merging) = &said.merging {
        let _ = ui.label(column.top(line), &Label::new(&merging.text));
        for file in &merging.files {
            let _ = ui.label(column.top(line), &Label::new(file).tone(Tone::Muted));
        }
    }
}

/// The height of everything the page lists.
fn content_height(state: &CompareState<'_>, (line, control, gap): (f32, f32, f32)) -> f32 {
    let mut rows = 2.0 * gap + line * 1.5 + control;
    if let CompareRead::Read(view) = state.read {
        let files = f32::from(u16::try_from(view.files.len()).unwrap_or(u16::MAX));
        rows += line * 5.0 + 2.0 * control + commits_height(state, line) + files * line;
        rows += line * 12.0;
    } else {
        rows += line;
    }
    rows * 1.2
}

/// The two sides and the arrows between them.
fn sides(state: &CompareState<'_>, ui: &mut Ui<'_>, row: Rect, intents: &mut Vec<Intent>) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    side(state, (&state.target.first, true), ui, &mut parts, intents);
    let swap_width = ui.theme().sizes.control_height * 1.5;
    intents.extend(ui.clip(parts.left(swap_width), |ui| swap(state, ui)));
    side(state, (&state.target.second, false), ui, &mut parts, intents);
}

/// A side: the button that chooses another in its place, its short hash and **gone**.
fn side(
    state: &CompareState<'_>,
    (side, is_first): (&CompareSide, bool),
    ui: &mut Ui<'_>,
    parts: &mut Area,
    intents: &mut Vec<Intent>,
) {
    let icon = match &side.side {
        Side::WorkingTree => icons::EDITED,
        Side::Commit(_) if !side.is_followed => icons::COMMIT,
        Side::Commit(_) => icons::BRANCH,
    };
    let tooltip = side.tooltip();
    let button = Button::secondary(&side.name)
        .kind(ButtonKind::Outline)
        .with_icon(icon)
        .with_tooltip(&tooltip);
    let (width, height) = ui.button_size(&button);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button(("side", is_first), cell, &button).clicked {
        intents.push(Intent::Compare(CompareIntent::ChangeSide(state.id, is_first)));
    }
    if let Side::Commit(commit) = &side.side
        && side.name != commit.short()
    {
        let hash = Button::secondary(commit.short()).kind(ButtonKind::Icon);
        let (width, height) = ui.button_size(&hash);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("side hash", is_first), cell, &hash).clicked {
            intents.push(Intent::Compare(CompareIntent::OpenCommit(state.id, commit.clone())));
        }
    }
    if side.is_gone {
        let width = ui.measure(words::GONE, ui.text_style(TextKind::Body));
        let _ = ui.label(parts.left(width), &Label::new(words::GONE).tone(Tone::Muted));
    }
}

/// A file that differs: its path, a click opening it in the Diff page, and the arrow that takes the version of the
/// second side, unless the second side is the working tree; **Compare with…** in its context menu
/// (`details.compare-file-with`).
fn file_row(
    state: &CompareState<'_>,
    (index, file): (usize, &model::state::FileChange),
    ui: &mut Ui<'_>,
    row: Rect,
    intents: &mut Vec<Intent>,
) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(row, spacing.medium);
    let second = &state.target.second;
    if !second.is_working_tree() {
        let tooltip = words::take_file(&second.name, file.kind == ChangeKind::Deleted);
        let take = Button::icon(icons::TAKE_INCOMING, &tooltip);
        let (width, height) = ui.button_size(&take);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("take file", index), cell, &take).clicked {
            intents.push(Intent::Compare(CompareIntent::TakeFile(state.id, index)));
        }
    }
    let label = match &file.original_path {
        Some(original) => format!("{original} \u{2192} {}", file.path),
        None => file.path.clone(),
    };
    let open = Button::secondary(&label).kind(ButtonKind::Icon).with_icon(icons::FILE);
    let rest = parts.rest();
    let (width, height) = ui.button_size(&open);
    let cell = place(rest, (width.min(rest.width), height), (Align::Start, Align::Center));
    let response = ui.button(("open file", index), cell, &open);
    if response.clicked {
        intents.push(Intent::Compare(CompareIntent::OpenFile(state.id, index)));
    }
    let entries = [MenuEntry::Item(MenuItem::new(crate::text::details::COMPARE_WITH, ()))];
    if ui.context_menu(("file menu", index), &response, &entries).is_some() {
        let list = FileList::Comparison {
            first: state.target.first.clone(),
            second: state.target.second.clone(),
        };
        intents.push(crate::details::compare_file(file, list));
    }
}
