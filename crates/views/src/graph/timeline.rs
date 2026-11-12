//! The timeline (`graph.view`), for now as a plain list of the commits read by `graph.read-commits`: two lines a row,
//! the subject, then the author and the date, with the short hash on the right.

use model::graph::Row;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Label, List, SelectionMode, TextKind, Tone, Ui};

use crate::intent::Intent;
use crate::text::dates::short_date;
use crate::text::timeline as words;

/// The side of the dot of a commit.
const DOT: f32 = 10.0;

/// What the timeline lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphState<'a> {
    /// The first commits are being read.
    Loading,
    /// The commits read, newest first.
    Listed {
        /// The rows.
        rows: &'a [Row],
        /// Whether the repository has no commit at all.
        has_no_commits: bool,
    },
}

/// The timeline, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it: `graph.view` (the lanes of `graph.lanes`, the labels of `graph.labels`, selecting a
/// commit for the inspector, opening it as a page), `graph.uncommitted-row` (the draft at its top), `graph.load-more`
/// (the next pages while scrolling), `graph.stashes`, `graph.fold-merge`, `graph.keyboard`, `filter.bar`.
pub fn view(state: &GraphState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.large + theme.sizes.spacing.small;
    let inner = inset(
        rect,
        Insets {
            left: padding,
            top: padding,
            right: padding,
            bottom: 0.0,
        },
    );
    ui.region(
        "timeline region",
        (rect, AccessRole::Main, words::TIMELINE),
        |ui| match *state {
            GraphState::Loading => say(ui, inner, words::LOADING_COMMITS),
            GraphState::Listed {
                rows: [],
                has_no_commits,
            } => {
                let said = if has_no_commits {
                    words::NO_COMMITS_YET
                } else {
                    words::NO_COMMITS_TO_SHOW
                };
                say(ui, inner, said);
            }
            GraphState::Listed { rows, .. } => history(rows, ui, inner),
        },
    );
    Vec::new()
}

/// A sentence in the middle of the timeline, in place of the commits.
fn say(ui: &mut Ui<'_>, rect: ui::Rect, words: &str) {
    let height = ui.theme().sizes.row_height;
    let line = place(rect, (rect.width, height), (Align::Center, Align::Center));
    let _ = ui.label(line, &Label::new(words).tone(Tone::Muted).align(Align::Center));
}

/// The commits, as a list that draws only the rows in sight.
fn history(rows: &[Row], ui: &mut Ui<'_>, rect: ui::Rect) {
    let list = List {
        name: words::HISTORY,
        row_count: rows.len(),
        row_height: ui.theme().sizes.timeline_row_height,
        selection_mode: SelectionMode::Single,
        lead: 0.0,
    };
    let _ = ui.list("history", rect, &list, |ui, row| {
        if let Some(commit) = rows.get(row.index).and_then(Row::commit) {
            commit_row(ui, row, commit);
        }
    });
}

/// A row: the dot of the commit, the subject over the author and the date, and the short hash on the right.
fn commit_row(ui: &mut Ui<'_>, row: &ui::Row, commit: &model::commit::Commit) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut parts = Area::with_gap(inset(row.rect, Insets::symmetric(spacing.large, 0.0)), spacing.large);
    let dot = parts.left(DOT);
    ui.fill_circle(dot.center(), DOT * 0.5, theme.lane(0));
    let hash = commit.id.short();
    let hash_width = ui.measure(hash, ui.text_style(TextKind::Hash));
    let _ = ui.label(
        parts.right(hash_width),
        &Label::new(hash).kind(TextKind::Hash).tone(Tone::Muted),
    );
    let text = parts.rest();
    let [first, second] = ui::layout::rows(text, 0.0, [ui::layout::Size::Fill(1.0), ui::layout::Size::Fill(1.0)]);
    let _ = ui.label(first, &Label::new(&commit.subject).kind(TextKind::Medium));
    let byline = words::author_and_date(&commit.author_name, &short_date(commit.author_date.0));
    let _ = ui.label(second, &Label::new(&byline).kind(TextKind::Detail).tone(Tone::Muted));
}
