//! What a repository or a worktree is on, as a row of **Repositories** says it on its right (`repo.worktree-summary`): `main ● 3 ↓2 ↑1`, what is not there not said.

use model::repo::{SummaryHead, WorktreeSummary};
use ui::layout::{Align, Area, place};
use ui::{Label, Point, Rect, TextKind, TextRun, Tone, Ui};

use crate::text::repo as words;

/// What a row says it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSummary {
    /// The summary, as read last.
    pub summary: WorktreeSummary,
    /// Whether the last read of it failed, and so it is the one before, drawn muted.
    pub is_stale: bool,
}

/// The words of what is checked out: the branch, `detached at 3f2a1c9`, `no commits yet` or `missing`.
#[must_use]
pub fn head_words(head: &SummaryHead) -> String {
    match head {
        SummaryHead::Branch(name) => name.clone(),
        SummaryHead::Detached(short) => words::detached_at(short),
        SummaryHead::NoCommits => crate::text::header::NO_COMMITS_YET.to_owned(),
        SummaryHead::Missing => words::MISSING.to_owned(),
    }
}

/// The counts of a summary, `● 3 ↓2 ↑1`, each only when it is not nothing; empty when there is nothing to say.
#[must_use]
pub fn counts_words(summary: &TabSummary) -> String {
    let summary = &summary.summary;
    let mut parts = Vec::new();
    if summary.changed > 0 {
        parts.push(words::changed_files(summary.changed));
    }
    let distance = words::to_pull_and_push(summary.behind(), summary.ahead());
    if !distance.is_empty() {
        parts.push(distance);
    }
    parts.join(" ")
}

/// How wide the line of a summary is, all of it.
pub(crate) fn summary_width(ui: &Ui<'_>, summary: &TabSummary) -> f32 {
    let style = ui.text_style(TextKind::Detail);
    let gap = ui.theme().sizes.spacing.small;
    let head = head_words(&summary.summary.head);
    let counts = counts_words(summary);
    let mut width = ui.measure(&head, style);
    if !counts.is_empty() {
        width += gap + ui.measure(&counts, style);
    }
    width
}

/// The line of a summary over a rectangle, in the small text of a second line: the branch, muted; how many files are
/// changed, in the color of changes; the commits to pull and to push, muted. A summary whose last read failed is all
/// muted.
pub(crate) fn summary_line(ui: &mut Ui<'_>, rect: Rect, summary: &TabSummary) {
    let theme = *ui.theme();
    let style = ui.text_style(TextKind::Detail);
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.small);
    let head = head_words(&summary.summary.head);
    let distance = words::to_pull_and_push(summary.summary.behind(), summary.summary.ahead());
    let changed = (summary.summary.changed > 0).then(|| words::changed_files(summary.summary.changed));
    let after: f32 = [changed.as_deref(), Some(distance.as_str())]
        .into_iter()
        .flatten()
        .filter(|text| !text.is_empty())
        .map(|text| ui.measure(text, style) + theme.sizes.spacing.small)
        .sum();
    let head_width = ui.measure(&head, style).min((rect.width - after).max(0.0));
    let _ = ui.label(
        parts.left(head_width),
        &Label::new(&head).kind(TextKind::Detail).tone(Tone::Muted),
    );
    if let Some(changed) = changed {
        let color = if summary.is_stale {
            theme.colors.muted_foreground
        } else {
            theme.colors.modified
        };
        let cell = parts.left(ui.measure(&changed, style));
        let line = place(cell, (cell.width, ui.line_height(style)), (Align::Start, Align::Center));
        let _ = ui.text(&TextRun {
            text: &changed,
            style,
            color,
            origin: Point::new(line.x, line.y),
            max_width: None,
        });
    }
    if !distance.is_empty() {
        let _ = ui.label(
            parts.rest(),
            &Label::new(&distance).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
}
