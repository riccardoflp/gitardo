//! The Stashes list of the branch rail, kept aside (`stash.pane`): its title with how many stashes there are and the
//! button that stashes all changes, how to make a stash while there is none, each stash with its message and how long
//! ago it was made, the buttons that apply, pop or drop it under the pointer, and, unfolded by its arrow, the files it
//! changed then the untracked files it took, each opening its diff.
//!
//! Its rows are rows of the rail, so that the filter box, **Show all `<n>`…** and the drawing of the rows in sight work
//! for it as for the other lists.

use std::collections::{BTreeMap, BTreeSet};

use model::CommitId;
use model::contract::{FeatureId, Target};
use model::details::CommitDetail;
use model::state::{FileChange, Stash};
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Icon, Label, Rect, Sense, TextKind, Tone, Ui};

use super::draw::{Context, RailReport, indent, list_title};
use super::entry::entry_row;
use super::rows::{Entry, ListHead, RailRow};
use crate::details::{FileLayout, file_name, file_start};
use crate::diff::{DiffIntent, commit_file, untracked_file};
use crate::feature::FeatureIntent;
use crate::graph::{StashEntry, stash_intent};
use crate::icons;
use crate::intent::{Intent, RailIntent, RailList};
use crate::text::dates::relative_date;
use crate::text::stash as words;

/// What the Stashes list shows of its stashes beyond the list the repository has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StashesShown<'a> {
    /// The stashes unfolded, by their commits.
    pub unfolded: &'a BTreeSet<CommitId>,
    /// What was read of the files of the stashes unfolded once, by their commits.
    pub files: &'a BTreeMap<CommitId, StashFilesRead>,
    /// Whether Gitardo is running an operation: the rows have no buttons then.
    pub is_operation_running: bool,
    /// The moment the ages of the stashes are said from, in seconds since 1970.
    pub now: i64,
}

/// How far the files of a stash are read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StashFilesRead {
    /// They are read.
    Loading,
    /// They are read: the stash as a commit, its files against the commit it was made on and its untracked files.
    Read(Box<CommitDetail>),
    /// Git could not read them, or the stash is gone.
    Failed,
}

/// A line under a stash unfolded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StashLine<'a> {
    /// **Loading…**, while its files are read.
    Loading,
    /// **No files**: it has none.
    NoFiles,
    /// Git could not read them.
    Failed,
    /// One of its files.
    File {
        /// The stash as a commit.
        detail: &'a CommitDetail,
        /// The file.
        file: &'a FileChange,
        /// Whether it is an untracked file the stash took, not a file it changed.
        is_untracked: bool,
    },
}

/// A row of the Stashes list, and where it is drawn.
#[derive(Clone, Copy)]
pub(super) struct StashesRow<'s, 'a> {
    /// What the rail is drawn from.
    pub(super) context: Context<'s, 'a>,
    /// The row.
    pub(super) row: &'s RailRow<'a>,
    /// Where it is, which may be partly out of sight.
    pub(super) rect: Rect,
}

/// A row of the Stashes list of the branch rail, over its rectangle: the title of the list with the button that stashes
/// all changes, a stash, a line of its files, or how to make a stash while there is none.
///
/// Feature: stash.pane
pub(super) fn stashes_view(state: &StashesRow<'_, '_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let (context, rect) = (state.context, state.rect);
    match state.row {
        RailRow::Kept(head) | RailRow::Heading(head) => title(ui, context, head, rect),
        RailRow::Entry { list, entry, marks } => {
            let mut report = RailReport::default();
            entry_row(ui, context, (list, entry, *marks), rect, &mut report);
            report.intents
        }
        RailRow::StashLine { stash, line } => stash_line(ui, (stash, *line), rect),
        RailRow::NoStashes => {
            let _ = ui.paragraph(empty_rect(ui, rect), words::NO_STASHES, (TextKind::Detail, Tone::Muted));
            Vec::new()
        }
        // The other rows are not of the Stashes list, and the rail draws them itself.
        RailRow::KeptTitle | RailRow::Matches { .. } | RailRow::InFull { .. } | RailRow::Gap => Vec::new(),
    }
}

/// Whether a row is one of the Stashes list, which [`stashes_view`] draws.
pub(super) fn is_of_stashes(row: &RailRow<'_>) -> bool {
    match row {
        RailRow::Kept(head) | RailRow::Heading(head) => head.list == RailList::Stashes,
        RailRow::Entry { list, .. } => *list == RailList::Stashes,
        RailRow::NoStashes | RailRow::StashLine { .. } => true,
        RailRow::KeptTitle | RailRow::Matches { .. } | RailRow::InFull { .. } | RailRow::Gap => false,
    }
}

/// The rows of the rail with those only the Stashes list has: how to make a stash under its title while it is open and
/// has none, and under each stash unfolded the lines of its files.
pub(super) fn with_stash_rows<'a>(rows: Vec<RailRow<'a>>, shown: &StashesShown<'a>) -> Vec<RailRow<'a>> {
    let mut all = Vec::with_capacity(rows.len());
    for row in rows {
        let added = match &row {
            RailRow::Kept(head) if head.list == RailList::Stashes && head.is_open && head.all == 0 => {
                vec![RailRow::NoStashes]
            }
            RailRow::Entry {
                entry: Entry::Stash(stash),
                ..
            } => file_lines(stash, shown),
            _ => Vec::new(), // No other row has rows of its own under it.
        };
        all.push(row);
        all.extend(added);
    }
    all
}

/// The lines under a stash, while it is unfolded: **Loading…** while its files are read, the files it changed then the
/// untracked files it took, or **No files**.
fn file_lines<'a>(stash: &'a Stash, shown: &StashesShown<'a>) -> Vec<RailRow<'a>> {
    if !shown.unfolded.contains(&stash.commit) {
        return Vec::new();
    }
    let line = |line| RailRow::StashLine { stash, line };
    match shown.files.get(&stash.commit) {
        None | Some(StashFilesRead::Loading) => vec![line(StashLine::Loading)],
        Some(StashFilesRead::Failed) => vec![line(StashLine::Failed)],
        Some(StashFilesRead::Read(detail)) => {
            let detail: &CommitDetail = detail;
            let changed = detail.files.iter().map(|file| (file, false));
            let untracked = detail.untracked_files.iter().map(|file| (file, true));
            let lines: Vec<RailRow<'a>> = changed
                .chain(untracked)
                .map(|(file, is_untracked)| {
                    line(StashLine::File {
                        detail,
                        file,
                        is_untracked,
                    })
                })
                .collect();
            if lines.is_empty() {
                vec![line(StashLine::NoFiles)]
            } else {
                lines
            }
        }
    }
}

/// The title of the list, with the button at its end that stashes all changes once there is a commit, under the pointer
/// and while the list is open with no stash, as the **…** of the other titles; it cannot be clicked while an operation
/// runs.
fn title(ui: &mut Ui<'_>, context: Context<'_, '_>, head: &ListHead, rect: Rect) -> Vec<Intent> {
    let is_pointed = ui.is_pointer_within(rect);
    let is_empty = head.is_open && head.all == 0;
    if context.state.repository.head.commit.is_none() || !(is_pointed || is_empty) {
        return list_title(ui, context, head, rect);
    }
    let theme = *ui.theme();
    if is_pointed {
        // The whole title is lit, its button too, as the other titles are with their **…**.
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let side = theme.sizes.row_height;
    let mut parts = Area::new(rect);
    let cell = parts.right(side + theme.sizes.spacing.small);
    let mut intents = list_title(ui, context, head, parts.rest());
    let mut button = Button::icon(icons::STASH, words::STASH_ALL).kind(ButtonKind::Icon);
    if context.state.stashes.is_operation_running {
        button = button.disabled();
    }
    let placed = place(cell, (side, side), (Align::Start, Align::Center));
    if ui.button("stash all changes", placed, &button).clicked {
        intents.push(Intent::Feature(FeatureIntent::Run(
            FeatureId("stash.push"),
            Target::Head,
        )));
    }
    intents
}

/// What a stash's row shows past the generic entry: the arrow that unfolds it, its message, and how long ago it was
/// made or, under the pointer and unless an operation runs, **Apply**, **Pop** and **Drop…**.
pub(super) fn stash_entry(
    ui: &mut Ui<'_>,
    context: Context<'_, '_>,
    stash: &Stash,
    (is_pointed, rect): (bool, Rect),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let shown = context.state.stashes;
    let mut intents = Vec::new();
    let side = rect.height;
    let start = rect.x + indent(&theme);
    let arrow = Rect::new(start - (side - theme.sizes.icon) * 0.5, rect.y, side, side);
    let is_unfolded = shown.unfolded.contains(&stash.commit);
    let (icon, tooltip) = if is_unfolded {
        (icons::UNFOLDED, words::FOLD)
    } else {
        (icons::FOLDED, words::UNFOLD)
    };
    let button = Button::icon(icon, tooltip).kind(ButtonKind::Icon);
    if ui.button(("stash arrow", &stash.commit), arrow, &button).clicked {
        intents.push(Intent::Rail(RailIntent::ToggleStash(stash.commit.clone())));
    }
    let words_start = start + theme.sizes.icon + spacing.small;
    let mut parts = Area::with_gap(
        Rect::new(
            words_start,
            rect.y,
            (rect.right() - spacing.medium - words_start).max(0.0),
            rect.height,
        ),
        spacing.small,
    );
    if is_pointed && !shown.is_operation_running {
        for (chosen, icon, tooltip) in STASH_BUTTONS {
            let button = Button::icon(icon, tooltip).kind(ButtonKind::Icon);
            if ui
                .button(("stash button", &stash.commit, tooltip), parts.right(side), &button)
                .clicked
            {
                intents.push(stash_intent(chosen, stash.index));
            }
        }
    } else {
        let age = relative_date(stash.date.0, shown.now);
        let width = ui.measure(&age, ui.text_style(TextKind::Small));
        let _ = ui.label(
            parts.right(width),
            &Label::new(&age).kind(TextKind::Small).tone(Tone::Muted),
        );
    }
    let _ = ui.label(parts.rest(), &Label::new(&stash.message).kind(TextKind::Detail));
    intents
}

/// A line under a stash unfolded: **Loading…**, **No files**, what git could not read, or a file, whose click opens
/// the Diff page of the file between the commit the stash was made on and the stash, or of an untracked file against
/// nothing.
fn stash_line(ui: &mut Ui<'_>, (stash, line): (&Stash, StashLine<'_>), rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let start = indent(&theme) + theme.sizes.icon + theme.sizes.spacing.small;
    let inner = Rect::new(rect.x + start, rect.y, (rect.width - start).max(0.0), rect.height);
    let said = match line {
        StashLine::Loading => words::LOADING,
        StashLine::NoFiles => words::NO_FILES,
        StashLine::Failed => words::NOT_READ,
        StashLine::File {
            detail,
            file,
            is_untracked,
        } => return file_row(ui, (stash, detail), (file, is_untracked), rect),
    };
    let _ = ui.label(inner, &Label::new(said).kind(TextKind::Detail).tone(Tone::Muted));
    Vec::new()
}

/// A file of a stash unfolded: its status, its icon, its name and its folder, lit under the pointer; a click opens its
/// diff.
fn file_row(
    ui: &mut Ui<'_>,
    (stash, detail): (&Stash, &CommitDetail),
    (file, is_untracked): (&FileChange, bool),
    rect: Rect,
) -> Vec<Intent> {
    let theme = *ui.theme();
    let id = ui.id(("stash file", &stash.commit, &file.path, is_untracked));
    let response = ui.interact(id, rect, Sense::CLICK);
    if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    // The status of the file lines up with the message of its stash; `file_start` keeps a margin of its own.
    let start = indent(&theme) + theme.sizes.icon + theme.sizes.spacing.small - theme.sizes.spacing.medium;
    let shifted = Rect::new(rect.x + start, rect.y, (rect.width - start).max(0.0), rect.height);
    let name_room = file_start(ui, shifted, (file, 0));
    file_name(ui, name_room, (file, FileLayout::List));
    if is_untracked {
        ui.tooltip(&response, words::UNTRACKED);
    }
    ui.describe(id, rect, &Description::new(AccessRole::Button, &file.path));
    if !response.clicked {
        return Vec::new();
    }
    let target = if is_untracked {
        untracked_file(detail, file)
    } else {
        Some(commit_file(detail, file))
    };
    target
        .map(|target| Intent::Diff(DiffIntent::Open(Box::new(target))))
        .into_iter()
        .collect()
}

/// Where the words of the list with no stash are drawn in their row.
fn empty_rect(ui: &Ui<'_>, rect: Rect) -> Rect {
    let theme = ui.theme();
    let start = indent(theme);
    let small = theme.sizes.spacing.small;
    Rect::new(
        rect.x + start,
        rect.y + small,
        (rect.width - start - theme.sizes.spacing.medium).max(1.0),
        (rect.height - 2.0 * small).max(0.0),
    )
}

/// The height of the words of the list with no stash, wrapped in a rail as wide as this.
pub(super) fn empty_height(ui: &Ui<'_>, width: f32) -> f32 {
    let style = ui.text_style(TextKind::Detail);
    let room = empty_rect(ui, Rect::new(0.0, 0.0, width, 0.0));
    let lines = ui.fonts().wrap(words::NO_STASHES, style, room.width).len();
    crate::line::rows_of(lines) * ui.line_height(style) + 2.0 * ui.theme().sizes.spacing.small
}

/// The buttons of a stash under the pointer, from the right: **Drop**, **Pop**, **Apply**.
const STASH_BUTTONS: [(StashEntry, Icon, &str); 3] = [
    (StashEntry::Drop, icons::DELETE, words::DROP),
    (StashEntry::Pop, icons::POP, words::POP),
    (StashEntry::Apply, icons::APPLY, words::APPLY),
];

/// The icon of **Drop** on a stash.
#[cfg(test)]
pub(crate) fn drop_stash_icon() -> Icon {
    STASH_BUTTONS
        .iter()
        .find(|(entry, _, _)| *entry == StashEntry::Drop)
        .map_or(icons::MORE, |(_, icon, _)| *icon)
}
