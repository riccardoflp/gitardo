//! The column of the blame before the text of a File page (`blame.file`): beside each line, the bar of its age and what
//! is said of its commit, the lines of the commit of the cursor brought out, with a mark on the scroll bar for each run
//! of them; the first line in sight names its commit when the first line of its run is scrolled away.

use std::ops::Range;

use model::blame::{BlameLine, PinnedInput, pick_pinned_line};
use ui::{AccessRole, Label, Rect, Sense, TextKind, Tone, Ui};

use super::{BlameIntent, FileBlame, FileBlameView, card, commit_of};
use crate::intent::Intent;
use crate::text::blame as words;

/// How wide the bar of the age of a line is.
const BAR_WIDTH: f32 = 3.0;

/// The column of the blame beside the lines in sight of the editor of a File page, drawn over the room the editor kept
/// for it, and the hover of a line; gives what a link of the hover asks.
///
/// Feature: blame.file
pub fn file_view(state: &FileBlameView<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let Some(file) = state.blame.file else {
        return Vec::new();
    };
    let places = state.places;
    let caret_commit = commit_of(file.lines.get(state.caret_line));
    let pinned = pick_pinned_line(&PinnedInput {
        lines: file.lines,
        texts: file.texts,
        first_in_sight: state.visible.start,
    });
    let mut intents = Vec::new();
    let mut hovered_row = None;
    ui.region(
        ("blame column", state.page),
        (places.gutter, AccessRole::Group, words::COLUMN),
        |ui| {
            ui.clip(places.gutter, |ui| {
                for line in state.visible.clone() {
                    let (top, height) = state.line_place(line);
                    let row = Rect::new(places.gutter.x, top, places.gutter.width, height);
                    let text = match &pinned {
                        Some((pinned_line, text)) if *pinned_line == line => text.as_str(),
                        _ => file.texts.get(line).map_or("", String::as_str),
                    };
                    let is_brought_out = caret_commit.is_some() && commit_of(file.lines.get(line)) == caret_commit;
                    draw_line(ui, (row, places.line_height), (&file, line), (text, is_brought_out));
                    if let Some(asked) = hover_of(ui, state, (row, line)) {
                        intents.push(asked);
                    }
                    if state.blame.hovered == Some(line) {
                        hovered_row = Some(row);
                    }
                }
            });
        },
    );
    if let Some(caret_commit) = caret_commit {
        scroll_marks(ui, state, (&file, caret_commit));
    }
    if let (Some(line), Some(row)) = (state.blame.hovered, hovered_row)
        && let Some(Some(blamed)) = file.lines.get(line)
    {
        intents.extend(card::hover_card(ui, (state.page, line, true), blamed, row));
    }
    intents
}

/// Draws what is beside a line, as tall as its rows: its bar, and its text on its first row, on the highlight when it is
/// of the commit of the cursor.
fn draw_line(
    ui: &mut Ui<'_>,
    (row, line_height): (Rect, f32),
    (file, line): (&FileBlame<'_>, usize),
    (text, is_brought_out): (&str, bool),
) {
    let theme = *ui.theme();
    let colors = theme.colors;
    if is_brought_out {
        ui.fill_rect(row, colors.list_inactive_selection);
    }
    if let Some(Some(color)) = file.bars.get(line) {
        ui.fill_rect(Rect::new(row.x, row.y, BAR_WIDTH, row.height), *color);
    }
    if text.is_empty() {
        return;
    }
    let left = row.x + BAR_WIDTH + theme.sizes.spacing.medium;
    let place = Rect::new(left, row.y, (row.right() - left).max(0.0), line_height);
    let tone = if is_brought_out { Tone::Normal } else { Tone::Muted };
    let _ = ui.label(place, &Label::new(text).kind(TextKind::Code).tone(tone));
}

/// The hover of a committed line: asked open once the pointer rests on what is beside it.
fn hover_of(ui: &mut Ui<'_>, state: &FileBlameView<'_>, (row, line): (Rect, usize)) -> Option<Intent> {
    let is_committed = commit_of(state.blame.file?.lines.get(line)).is_some();
    let id = ui.id(("blame line", state.page, line));
    let response = ui.interact(id, row, Sense::HOVER);
    let rests = ui
        .hovered_since(&response)
        .is_some_and(|since| ui.now().saturating_sub(since) >= card::REST);
    if !rests || !is_committed || state.blame.hovered == Some(line) {
        if response.hovered && !rests {
            ui.request_frame_at(ui.now() + card::REST);
        }
        return None;
    }
    Some(Intent::Blame(BlameIntent::Hover(state.page, Some(line))))
}

/// A mark on the scroll bar of a file longer than the page for each run of lines of the commit of the cursor.
fn scroll_marks(ui: &mut Ui<'_>, state: &FileBlameView<'_>, (file, commit): (&FileBlame<'_>, &model::CommitId)) {
    let places = state.places;
    let rows = state.rows.count();
    let content = crate::number::float_of_count(rows) * places.line_height;
    if content <= places.text.height || rows == 0 {
        return;
    }
    let track = places.track;
    let scale = track.height / crate::number::float_of_count(rows);
    let color = ui.theme().colors.foreground;
    for run in runs_of(file.lines, commit) {
        let (start, end) = (state.rows.first_row(run.start), state.rows.first_row(run.end));
        let top = track.y + crate::number::float_of_count(start) * scale;
        let height = (crate::number::float_of_count(end - start) * scale).max(2.0);
        ui.fill_rect(Rect::new(track.x, top, track.width, height), color);
    }
}

/// The runs of lines of a commit that follow each other, from 0: one mark on the scroll bar for each.
#[must_use]
pub fn runs_of(lines: &[Option<BlameLine>], commit: &model::CommitId) -> Vec<Range<usize>> {
    let mut runs: Vec<Range<usize>> = Vec::new();
    for (line, blamed) in lines.iter().enumerate() {
        if commit_of(Some(blamed)) != Some(commit) {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.end == line => run.end = line + 1,
            _ => runs.push(line..line + 1),
        }
    }
    runs
}
