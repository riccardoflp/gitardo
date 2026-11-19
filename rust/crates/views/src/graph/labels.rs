//! The labels of a commit at the start of the line of its message (`graph.labels`, `graph.label-overflow`): as many as
//! fit whole in the room its message leaves them, in their order, then one label `+N` for the others; resting on them
//! opens the list of all of them when some are not shown whole. The page of a commit draws them too.

use std::borrow::Cow;
use std::time::Duration;

use model::CommitId;
use model::graph::{Label, LabelKind};
use ui::{AccessRole, Badge, BadgeFill, Icon, Rect, Sense, Tone, Ui};

use super::intent::{HoverTarget, HoverWay};
use super::label_overflow::{LabelList, OPEN_AFTER};
use super::pull_request_badge::{after_label, badge_room};
use super::row::{RowContext, RowReport};
use crate::text::timeline as words;

/// The share of the line of a message that the message keeps, when it is that long, before the labels take the rest:
/// the labels have at least the other two fifths of it (Q640).
pub(super) const MESSAGE_SHARE: f32 = 0.6;

/// How many times its own height a label shortened beside `+N` is at least wide, so that its icon and a few letters
/// are seen; with less room only `+N` is shown.
const SHORTENED_LEAST: f32 = 3.0;

/// The marks of the tags a remote lacks and the commit whose labels are drawn, for the icons and the tooltip of the
/// label of a tag (`tag.remote-marks`); none before a check, and on the page of a commit.
pub(crate) type TagsAt<'a> = Option<(&'a model::tag::TagMarks, &'a CommitId)>;

/// A label as a pill: its words, its icons and how it is filled, or the count of the labels not shown.
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
    pub(super) is_count: bool,
}

impl Pill<'_> {
    /// The pill drawn in the color of the commit's line.
    pub(super) fn badge(&self, color: usize) -> Badge<'_> {
        Badge {
            text: &self.text,
            icons: self.icons,
            tone: if self.is_count { Tone::Muted } else { Tone::Lane(color) },
            fill: self.fill,
        }
    }
}

/// A pill fitted in the room of a row.
pub(super) struct Fitted<'a> {
    /// The pill.
    pub(super) pill: Pill<'a>,
    /// The width it is given, what follows it on its label included and the gap after it left out: less than it needs
    /// when it is shortened.
    pub(super) width: f32,
    /// The place of its label among those of the commit; none for the count.
    pub(super) label: Option<usize>,
}

/// The labels of a commit fitted in the room of its row.
pub(super) struct FittedLabels<'a> {
    /// The pills, from the left.
    pub(super) pills: Vec<Fitted<'a>>,
    /// The width they take, with the gap after each.
    pub(super) width: f32,
    /// Whether some labels are not shown whole: counted in `+N`, or the first one shortened.
    pub(super) hides_some: bool,
}

/// The room labels are fitted in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Room {
    /// How wide it is.
    pub(super) width: f32,
    /// How wide a label shortened beside `+N` is at least.
    pub(super) least: f32,
    /// The gap after each label.
    pub(super) gap: f32,
}

/// The pills of the labels of a commit in a room, each as wide as `width_of` says of it and of the place of its label:
/// as many whole as fit, in their order, and the count of the others, `+3`; when not even the first fits whole beside
/// the count, the first shortened to the room left beside it, when that is at least `least`; else the count alone;
/// none when not even that fits.
pub(super) fn fitted_pills<'a>(
    labels: &'a [Label],
    (room, at): (Room, TagsAt<'_>),
    width_of: impl Fn(&Pill<'_>, Option<usize>) -> f32,
) -> FittedLabels<'a> {
    let fit = |named: usize| -> FittedLabels<'a> {
        let pills: Vec<Fitted<'a>> = counted_pills(labels, named, at)
            .into_iter()
            .enumerate()
            .map(|(place, pill)| {
                let label = (!pill.is_count).then_some(place);
                let width = width_of(&pill, label);
                Fitted { pill, width, label }
            })
            .collect();
        let width = pills.iter().map(|fitted| fitted.width + room.gap).sum();
        FittedLabels {
            pills,
            width,
            hides_some: named < labels.len(),
        }
    };
    for named in (1..=labels.len()).rev() {
        let fitted = fit(named);
        if fitted.width <= room.width {
            return fitted;
        }
    }
    let mut first = fit(1);
    let beside: f32 = first.pills.iter().skip(1).map(|fitted| fitted.width + room.gap).sum();
    let left = room.width - beside - room.gap;
    if let Some(shortened) = first.pills.first_mut().filter(|_| left >= room.least) {
        shortened.width = left.min(shortened.width);
        first.width = left + room.gap + beside;
        first.hides_some = true;
        return first;
    }
    let count = fit(0);
    if !labels.is_empty() && count.width <= room.width {
        return count;
    }
    FittedLabels {
        pills: Vec::new(),
        width: 0.0,
        hides_some: !labels.is_empty(),
    }
}

/// The labels of a commit drawn on its row.
#[derive(Clone, Copy, Debug)]
pub(super) struct RowLabels<'a> {
    /// The row shown, from the top.
    pub(super) shown: usize,
    /// The commit.
    pub(super) commit: &'a CommitId,
    /// The color of its line.
    pub(super) color: usize,
}

/// Draws the labels of a commit from the start of their room on the line of its message, for the screen reader a list
/// of labels; gives where the message starts. Resting on one of them, when some are not shown whole, asks for the list
/// of all of them (`graph.label-overflow`).
pub(super) fn row_labels(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    row: RowLabels<'_>,
    room: Rect,
    report: &mut RowReport,
) -> f32 {
    let state = context.state;
    let labels = state.layout.labels.get(row.commit).map_or(&[][..], Vec::as_slice);
    if labels.is_empty() {
        return room.x;
    }
    let gap = ui.theme().sizes.spacing.medium;
    let fit = Room {
        width: room.width,
        least: SHORTENED_LEAST * ui.badge_height(),
        gap,
    };
    let at = state.tag_marks.map(|marks| (marks, row.commit));
    let fitted = fitted_pills(labels, (fit, at), |pill, label| {
        let follows = label.and_then(|label| labels.get(label));
        ui.badge_width(&pill.badge(row.color)) + follows.map_or(0.0, |label| badge_room(ui, state, label, row.color))
    });
    let strip = Rect::new(room.x, room.y, (fitted.width - gap).max(0.0), room.height);
    let drawn = ui.region(
        ("labels", row.commit.as_str()),
        (strip, AccessRole::List, words::LABELS),
        |ui| draw_fitted(ui, context, (row, &fitted, room), report),
    );
    let is_open = state.label_list.is_some_and(|list| &list.commit == row.commit);
    if fitted.hides_some && is_open {
        let from = state.label_list.map_or(0, |list| list.from);
        let opener = drawn.get(from).or(drawn.last()).copied();
        report.open_anchor = opener.map(|opener| (row.shown, opener, strip));
    }
    room.x + fitted.width
}

/// Draws the pills fitted from the start of their room, each followed by the badge of its pull request when there is
/// room for it; gives where each was drawn.
fn draw_fitted(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (row, fitted, room): (RowLabels<'_>, &FittedLabels<'_>, Rect),
    report: &mut RowReport,
) -> Vec<Rect> {
    let labels = context
        .state
        .layout
        .labels
        .get(row.commit)
        .map_or(&[][..], Vec::as_slice);
    let gap = ui.theme().sizes.spacing.medium;
    let mut drawn_rects = Vec::new();
    let mut x = room.x;
    for (place, pill) in fitted.pills.iter().enumerate() {
        let badge = pill.pill.badge(row.color);
        let width = ui.badge_width(&badge).min(pill.width);
        // A label with less room than its own height is not drawn at all, rather than squeezed past the end.
        if width < room.height {
            break;
        }
        let drawn = ui.badge(Rect::new(x, room.y, width, room.height), &badge);
        drawn_rects.push(drawn);
        let since = hover_label(ui, (drawn, pill), row.commit, report);
        if fitted.hides_some {
            ask_list(ui, context, (row.commit, place), since, report);
        }
        if let Some(label) = pill.label {
            let end = x + pill.width;
            let _next = after_label(
                ui,
                context.state,
                (labels, label),
                (drawn, row.color, end),
                &mut report.intents,
            );
        }
        x += pill.width + gap;
    }
    drawn_rects
}

/// Notes the label under the pointer, and the one the pointer is on, and gives a label its tooltip; gives since when the
/// pointer rests on it.
fn hover_label(
    ui: &mut Ui<'_>,
    (rect, fitted): (Rect, &Fitted<'_>),
    commit: &CommitId,
    report: &mut RowReport,
) -> Option<Duration> {
    let id = ui.id(("label", commit.as_str(), fitted.pill.text.as_ref()));
    let response = ui.interact(id, rect, Sense::HOVER);
    if let Some(label) = fitted.label.filter(|_| ui.is_pointed_at(&response)) {
        report.pointed = Some((commit.clone(), label));
    }
    if let Some(tooltip) = &fitted.pill.tooltip {
        ui.tooltip(&response, tooltip);
    }
    let since = ui.hovered_since(&response)?;
    report.label = Some((hover_target(ui, commit), since));
    Some(since)
}

/// The history a label of a commit brings out: what it reaches, or with `Shift` held what it has that the commit
/// checked out has not.
pub(super) fn hover_target(ui: &Ui<'_>, commit: &CommitId) -> HoverTarget {
    let way = if ui.modifiers().shift() {
        HoverWay::NotCheckedOut
    } else {
        HoverWay::Reaches
    };
    HoverTarget {
        commit: commit.clone(),
        way,
    }
}

/// Asks for the list of the labels of a commit once the pointer has rested on one of its labels long enough, unless the
/// list is open for it already or was closed while the pointer rested there.
fn ask_list(
    ui: &mut Ui<'_>,
    context: &RowContext<'_>,
    (commit, place): (&CommitId, usize),
    since: Option<Duration>,
    report: &mut RowReport,
) {
    let state = context.state;
    let Some(since) = since else {
        return;
    };
    let is_open = state.label_list.is_some_and(|list| &list.commit == commit);
    let was_closed = state.label_list_closed_at.is_some_and(|closed| since < closed);
    if is_open || was_closed {
        return;
    }
    if ui.now().saturating_sub(since) >= OPEN_AFTER {
        report.open_labels = Some(LabelList {
            commit: commit.clone(),
            from: place,
        });
    } else {
        ui.request_frame_at(since + OPEN_AFTER);
    }
}

/// The pills of the first labels of a commit, then the count of the others when there are some.
pub(super) fn counted_pills<'a>(labels: &'a [Label], named: usize, at: TagsAt<'_>) -> Vec<Pill<'a>> {
    let mut pills: Vec<Pill<'a>> = labels.iter().take(named).map(|label| pill(label, at)).collect();
    if let Some(rest) = labels.get(named..).filter(|rest| !rest.is_empty()) {
        pills.push(Pill {
            text: Cow::Owned(format!("+{}", rest.len())),
            icons: &[],
            fill: BadgeFill::Tint,
            tooltip: None,
            is_count: true,
        });
    }
    pills
}

/// The pill of a label; the label of a tag with the mark of the remote checked after its icon, and its tooltip.
pub(super) fn pill<'a>(label: &'a Label, at: TagsAt<'_>) -> Pill<'a> {
    let (icons, tooltip): (&'static [Icon], Option<String>) = match &label.kind {
        LabelKind::Branch {
            worktree: Some(folder), ..
        } => (
            &[crate::icons::WORKTREE],
            Some(words::checked_out_in(&folder.display().to_string())),
        ),
        LabelKind::Branch { upstream: Some(_), .. } => (&[crate::icons::BRANCH, crate::icons::REMOTE], None),
        LabelKind::Head | LabelKind::Branch { .. } => (&[], None),
        LabelKind::RemoteBranch => (&[crate::icons::REMOTE], None),
        LabelKind::Tag => {
            let mark = at.and_then(|(marks, commit)| crate::tag::remote_marks_view(Some(marks), (&label.name, commit)));
            let (icon, tooltip) = mark.unzip();
            (crate::tag::label_icons(icon), tooltip)
        }
        LabelKind::Stash { .. } => (&[crate::icons::STASH], None),
        LabelKind::Bisect(mark) => crate::bisect::marks_view(*mark),
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

/// Draws the labels of a commit as pills from the left of a rectangle, in the color of its line, as its row does: the
/// page of a commit shows them so (`commit-page.view`); a label with less room than its own height is left out, as in
/// a row.
pub(crate) fn draw_labels(ui: &mut Ui<'_>, rect: Rect, (labels, color): (&[Label], usize)) {
    let gap = ui.theme().sizes.spacing.medium;
    let mut x = rect.x;
    for pill in &counted_pills(labels, labels.len(), None) {
        let badge = pill.badge(color);
        let width = ui.badge_width(&badge).min((rect.right() - x).max(0.0));
        if width < rect.height {
            break;
        }
        let _ = ui.badge(Rect::new(x, rect.y, width, rect.height), &badge);
        x += width + gap;
    }
}
