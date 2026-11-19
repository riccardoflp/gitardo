//! The page of what a rewrite changed, commit by commit (`range-diff.page`): the two lines compared, the count of each
//! kind of pair, and a row for each pair, a changed one opening to show its diff of diffs.

use model::CommitId;
use model::range_diff::{Pair, PairKind, RangeDiff, RangeDiffTarget, heading_of};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Key, KeyEvent, Label, Modifiers, NamedKey, Rect, TextKind, Tone, Ui};

use crate::icons;
use crate::intent::Intent;
use crate::text::range_diff as words;

/// The widest the column of the page is.
const COLUMN_WIDTH: f32 = 960.0;

/// How far the pairs are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeDiffRead<'a> {
    /// Not read yet: **Comparing…**.
    Comparing,
    /// Read.
    Read(&'a RangeDiff),
    /// The read failed: what git said.
    Failed(&'a str),
}

/// What the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RewriteState<'a> {
    /// What is compared, with the title.
    pub target: &'a RangeDiffTarget,
    /// Its name in its tab and its title row: `feature · what changed`.
    pub title: &'a str,
    /// How far the pairs are read.
    pub read: RangeDiffRead<'a>,
    /// The pairs opened, by their place in the list.
    pub opened: &'a [usize],
    /// The pair under the keyboard, which `↑` and `↓` move.
    pub lit: Option<usize>,
}

/// What the page asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RangeDiffIntent {
    /// Open a changed pair to show its diff of diffs, or close it.
    Toggle(usize),
    /// Put the keyboard on a pair.
    Light(usize),
    /// Open the page of a commit (`commit-page.open`).
    OpenCommit(CommitId),
}

/// A row of a pair, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairRow<'a> {
    /// How the two commits are paired.
    pub kind: PairKind,
    /// Whether it is dimmed: a pair the same (Q1141).
    pub is_dimmed: bool,
    /// The short hash before, none for one added.
    pub before: Option<&'a str>,
    /// The short hash now, none for one dropped.
    pub now: Option<&'a str>,
    /// The subject.
    pub subject: &'a str,
    /// What changed in a changed one: **message**, **changes** or both.
    pub changed: Option<String>,
}

/// The row of a pair.
#[must_use]
pub fn pair_row(pair: &Pair) -> PairRow<'_> {
    let changed = if pair.kind == PairKind::Changed {
        let parts = pair.changed_parts();
        words::changed_parts(parts.message, parts.changes)
    } else {
        None
    };
    PairRow {
        kind: pair.kind,
        is_dimmed: pair.kind == PairKind::Same,
        before: pair.before.as_ref().map(CommitId::short),
        now: pair.now.as_ref().map(CommitId::short),
        subject: &pair.subject,
        changed,
    }
}

/// How a line of a diff of diffs is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffTone {
    /// Only before, in the color of lines lost.
    Lost,
    /// Only now, in the color of lines gained.
    Gained,
    /// A heading: `## Commit message ##`, `## <path> ##`.
    Heading,
    /// A line around them, the same on both sides.
    Around,
}

/// The lines of the diff of diffs of a pair, each with how it is shown and its text without the outer mark.
#[must_use]
pub fn diff_lines(pair: &Pair) -> Vec<(DiffTone, &str)> {
    pair.diff
        .iter()
        .map(|line| {
            let text = line.get(1..).unwrap_or_default();
            let tone = match line.chars().next() {
                Some('-') => DiffTone::Lost,
                Some('+') => DiffTone::Gained,
                _ if heading_of(line).is_some() => DiffTone::Heading,
                _ => DiffTone::Around,
            };
            (tone, text)
        })
        .collect()
}

/// What a key asks of the page: `↑` and `↓` move through the rows, `Enter` and `→` open the row under the keyboard,
/// `←` closes it.
#[must_use]
pub fn keyed(state: &RewriteState<'_>, key: KeyEvent) -> Option<RangeDiffIntent> {
    let RangeDiffRead::Read(read) = state.read else {
        return None;
    };
    let last = read.pairs.len().checked_sub(1)?;
    if key.modifiers != Modifiers::NONE {
        return None;
    }
    let is_open = |row: usize| state.opened.contains(&row);
    match (key.key, state.lit) {
        (Key::Named(NamedKey::ArrowDown), None) => Some(RangeDiffIntent::Light(0)),
        (Key::Named(NamedKey::ArrowDown), Some(lit)) => (lit < last).then_some(RangeDiffIntent::Light(lit + 1)),
        (Key::Named(NamedKey::ArrowUp), Some(lit)) => (lit > 0).then(|| RangeDiffIntent::Light(lit - 1)),
        (Key::Named(NamedKey::Enter), Some(lit)) => Some(RangeDiffIntent::Toggle(lit)),
        (Key::Named(NamedKey::ArrowRight), Some(lit)) => (!is_open(lit)).then_some(RangeDiffIntent::Toggle(lit)),
        (Key::Named(NamedKey::ArrowLeft), Some(lit)) => is_open(lit).then_some(RangeDiffIntent::Toggle(lit)),
        _ => None,
    }
}

/// The page, over the rectangle its caller clipped it to: the title of what was compared, the two lines and the count
/// of each kind of pair, then a row for each pair: an icon for its kind, the short hashes before and now, each opening
/// its commit, the subject, which opens a changed pair, and what changed in it; under an opened one, its diff of diffs.
///
/// Feature: range-diff.page
pub fn page(state: &RewriteState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = theme.sizes.row_height;
    let width = COLUMN_WIDTH.min((rect.width - 2.0 * spacing.huge).max(0.0));
    let column_area = Rect::new(rect.x + (rect.width - width) * 0.5, rect.y, width, rect.height);
    let height = content_height(state, line) + 2.0 * spacing.large;
    for key in ui.unclaimed_keys() {
        intents.extend(keyed(state, key).map(Intent::RangeDiff));
    }
    ui.region("what changed", (rect, AccessRole::Main, state.title), |ui| {
        let _ = ui.scroll_area("what changed page", column_area, (width, height), |ui, content| {
            let mut column = Area::with_gap(inset(content, Insets::symmetric(0.0, spacing.large)), spacing.small);
            let _ = ui.heading(column.top(line * 1.5), &state.target.title);
            let read = match state.read {
                RangeDiffRead::Read(read) => read,
                RangeDiffRead::Comparing => {
                    let _ = ui.label(column.top(line), &Label::new(words::COMPARING).tone(Tone::Muted));
                    return;
                }
                RangeDiffRead::Failed(said) => {
                    let _ = ui.label(column.top(line), &Label::new(said).tone(Tone::Error));
                    return;
                }
            };
            let ranges = &state.target.ranges;
            for (label, count, from) in [
                (words::BEFORE, read.before_count, &ranges.before.from),
                (words::NOW, read.now_count, &ranges.now.from),
            ] {
                let text = words::line(label, count, short(from));
                let _ = ui.label(column.top(line), &Label::new(&text));
            }
            let _ = ui.label(column.top(line), &Label::new(&read.counts()).tone(Tone::Muted));
            for (index, pair) in read.pairs.iter().enumerate() {
                pair_line(state, (index, pair), ui, column.top(line), &mut intents);
                if state.opened.contains(&index) {
                    for (place, (tone, text)) in diff_lines(pair).into_iter().enumerate() {
                        diff_line(ui, column.top(line), (tone, text), (index, place));
                    }
                }
            }
        });
    });
    intents
}

/// A commit named by its full hash is shortened; a name is kept.
fn short(from: &str) -> &str {
    CommitId::parse(from).map_or(from, |_| from.get(..7).unwrap_or(from))
}

/// The height of everything the page lists.
fn content_height(state: &RewriteState<'_>, line: f32) -> f32 {
    let rows = match state.read {
        RangeDiffRead::Read(read) => {
            let opened: usize = state
                .opened
                .iter()
                .filter_map(|index| read.pairs.get(*index))
                .map(|pair| pair.diff.len())
                .sum();
            3 + read.pairs.len() + opened
        }
        RangeDiffRead::Comparing | RangeDiffRead::Failed(_) => 1,
    };
    let rows = f32::from(u16::try_from(rows).unwrap_or(u16::MAX));
    (line * 1.5 + rows * line) * 1.2
}

/// The row of a pair.
fn pair_line(
    state: &RewriteState<'_>,
    (index, pair): (usize, &Pair),
    ui: &mut Ui<'_>,
    row: Rect,
    intents: &mut Vec<Intent>,
) {
    let spacing = ui.theme().sizes.spacing;
    let shown = pair_row(pair);
    let mut parts = Area::with_gap(row, spacing.medium);
    let (icon, tooltip) = match shown.kind {
        PairKind::Same => (icons::ACCEPT_SIMPLE, words::SAME),
        PairKind::Changed => (icons::EDITED, words::CHANGED),
        PairKind::Added => (icons::ADD, words::ADDED),
        PairKind::Dropped => (icons::DELETE, words::DROPPED),
    };
    let tone = if shown.is_dimmed { Tone::Muted } else { Tone::Normal };
    let kind = Button::icon(icon, tooltip);
    let (width, height) = ui.button_size(&kind);
    let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
    if ui.button(("pair kind", index), cell, &kind).clicked && shown.kind == PairKind::Changed {
        intents.push(Intent::RangeDiff(RangeDiffIntent::Toggle(index)));
    }
    for (side, (short, commit)) in [(shown.before, &pair.before), (shown.now, &pair.now)]
        .into_iter()
        .enumerate()
    {
        if side == 1 {
            let arrow = "\u{2192}";
            let arrow_width = ui.measure(arrow, ui.text_style(TextKind::Body));
            let _ = ui.label(parts.left(arrow_width), &Label::new(arrow).tone(Tone::Muted));
        }
        let hash_width = ui.measure("0000000", ui.text_style(TextKind::Hash)) + spacing.medium;
        let cell = parts.left(hash_width);
        if let (Some(short), Some(commit)) = (short, commit) {
            let hash = Button::secondary(short).kind(ButtonKind::Icon);
            let (width, height) = ui.button_size(&hash);
            let cell = place(cell, (width.min(cell.width), height), (Align::Start, Align::Center));
            if ui.button(("pair hash", index, side), cell, &hash).clicked {
                intents.push(Intent::RangeDiff(RangeDiffIntent::OpenCommit(commit.clone())));
            }
        }
    }
    if let Some(changed) = &shown.changed {
        let changed_width = ui.measure(changed, ui.text_style(TextKind::Body));
        let _ = ui.label(parts.right(changed_width), &Label::new(changed).tone(Tone::Muted));
    }
    let subject_tone = if state.lit == Some(index) { Tone::Accent } else { tone };
    let rest = parts.rest();
    if shown.kind == PairKind::Changed {
        let open = Button::secondary(shown.subject).kind(ButtonKind::Icon);
        let (width, height) = ui.button_size(&open);
        let cell = place(rest, (width.min(rest.width), height), (Align::Start, Align::Center));
        if ui.button(("pair subject", index), cell, &open).clicked {
            intents.push(Intent::RangeDiff(RangeDiffIntent::Toggle(index)));
        }
    } else {
        let _ = ui.label(rest, &Label::new(shown.subject).tone(subject_tone));
    }
}

/// A line of a diff of diffs, in the colors of a diff.
fn diff_line(ui: &mut Ui<'_>, row: Rect, (tone, text): (DiffTone, &str), _key: (usize, usize)) {
    let indent = ui.theme().sizes.spacing.huge;
    let rect = Rect::new(row.x + indent, row.y, (row.width - indent).max(0.0), row.height);
    let label = match tone {
        DiffTone::Lost => Label::new(text).kind(TextKind::Code).tone(Tone::Error),
        DiffTone::Gained => Label::new(text).kind(TextKind::Code).tone(Tone::Success),
        DiffTone::Heading => Label::new(text.trim_start()).kind(TextKind::Strong),
        DiffTone::Around => Label::new(text).kind(TextKind::Code).tone(Tone::Muted),
    };
    let _ = ui.label(rect, &label);
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
