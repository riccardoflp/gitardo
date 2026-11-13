//! The parts of the comparison page that change how it compares: the arrows that swap the sides, the mode, the box
//! that narrows it, and the commits only in each side.

use model::compare::{COMMIT_LIMIT, CommitSide, CompareMode, CompareView, ComparedCommit};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Icon, InputBox, Key, Label, NamedKey, Rect, TextKind, Tone, Ui};

use super::{CompareIntent, CompareRead, CompareState};
use crate::intent::Intent;
use crate::text::compare as words;

/// The most commits a group lists.
const LISTED: usize = 300;

/// The arrows between the two sides, which compare in the other direction: the mode and the path kept.
///
/// Feature: compare.swap
pub fn swap(state: &CompareState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::icon(Icon::Refresh, words::SWAP_TOOLTIP);
    let (width, height) = ui.button_size(&button);
    let cell = place(rect, (width, height), (Align::Center, Align::Center));
    let response = ui.button(("swap", state.id.0), cell, &button);
    if response.clicked {
        vec![Intent::Compare(CompareIntent::Swap(state.id))]
    } else {
        Vec::new()
    }
}

/// **Since they diverged** and **Everything that differs**, the one shown pressed in, the first disabled when the
/// histories never meet; then **Open in the Diff page** and **Copy as patch** when files differ.
///
/// Feature: compare.set-mode
pub fn set_mode(state: &CompareState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(rect, spacing.medium);
    let (first, second) = (&state.target.first.name, &state.target.second.name);
    let view = match state.read {
        CompareRead::Read(view) => Some(view),
        CompareRead::Comparing | CompareRead::Failed => None,
    };
    let never_meet = view.is_some_and(|view| view.merge_base.is_none());
    for (mode, label) in [
        (CompareMode::SinceDiverged, words::SINCE_DIVERGED),
        (CompareMode::Everything, words::EVERYTHING),
    ] {
        let is_since = mode == CompareMode::SinceDiverged;
        let tooltip = if is_since && never_meet {
            words::NEVER_MEET.to_owned()
        } else {
            words::mode_tooltip(first, second, is_since)
        };
        let kind = mode_button_kind(state.target.mode, mode);
        let mut button = Button::secondary(label).kind(kind).with_tooltip(&tooltip);
        if is_since && never_meet {
            button = button.disabled();
        }
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.left(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("mode", is_since), cell, &button).clicked && state.target.mode != mode {
            intents.push(Intent::Compare(CompareIntent::SetMode(state.id, mode)));
        }
    }
    if view.is_some_and(|view| !view.files.is_empty()) {
        let copy = Button::secondary(words::COPY_AS_PATCH)
            .kind(ButtonKind::Outline)
            .with_tooltip(words::COPY_AS_PATCH_TOOLTIP);
        let open = Button::secondary(words::OPEN_IN_DIFF).kind(ButtonKind::Outline);
        for (key, button, intent) in [
            ("copy patch", copy, CompareIntent::CopyPatch(state.id)),
            ("open all", open, CompareIntent::OpenAll(state.id)),
        ] {
            let (width, height) = ui.button_size(&button);
            let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
            if ui.button(key, cell, &button).clicked {
                intents.push(Intent::Compare(intent));
            }
        }
    }
    intents
}

/// How the button of a mode is drawn: pressed in, filled, when it is the mode shown, as an outline otherwise.
#[must_use]
pub(crate) fn mode_button_kind(shown: CompareMode, mode: CompareMode) -> ButtonKind {
    if shown == mode {
        ButtonKind::Secondary
    } else {
        ButtonKind::Outline
    }
}

/// The box that narrows the comparison to a file or a folder: `Enter` narrows to what is typed, `Escape` puts back the
/// path in use, and **×** shows every file again.
///
/// Feature: compare.narrow-to-path
pub fn narrow_to_path(state: &CompareState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(rect, spacing.medium);
    if state.target.path.is_some() {
        let clear = Button::icon(Icon::Close, words::SHOW_EVERY_FILE);
        let (width, height) = ui.button_size(&clear);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("clear path", state.id.0), cell, &clear).clicked {
            intents.push(Intent::Compare(CompareIntent::ClearPath(state.id)));
        }
    }
    let field = parts.rest();
    let typed = ui.input_box(
        ("narrow", state.id.0),
        field,
        &InputBox::new(state.typed_path, words::NARROW_PLACEHOLDER),
    );
    if let Some(text) = typed.changed_text {
        intents.push(Intent::Compare(CompareIntent::TypePath(state.id, text)));
    }
    if typed.is_submitted {
        intents.push(Intent::Compare(CompareIntent::Narrow(state.id)));
    }
    let escaped = typed.response.has_focus
        && ui
            .key_presses()
            .iter()
            .any(|key| key.key == Key::Named(NamedKey::Escape));
    if escaped {
        intents.push(Intent::Compare(CompareIntent::ResetPath(state.id)));
    }
    intents
}

/// The rows of a group of commits: how many commits it lists, whether **Show all N commits…** follows them, and
/// whether **The newest N are listed.** does, when the side has more commits than were read.
fn group(state: &CompareState<'_>, view: &CompareView, side: CommitSide) -> (usize, bool, bool) {
    let read = view.commits.iter().filter(|commit| commit.side == side).count();
    let count = match side {
        CommitSide::First => view.only_in_first,
        CommitSide::Second => view.only_in_second,
    };
    let listed = if state.is_all_commits_shown {
        read
    } else {
        read.min(LISTED)
    };
    (listed, listed < read, count > read)
}

/// The height of **Commits** and its two groups, nothing when neither side has commits of its own.
#[must_use]
pub fn commits_height(state: &CompareState<'_>, line: f32) -> f32 {
    let CompareRead::Read(view) = state.read else {
        return 0.0;
    };
    if view.commits.is_empty() {
        return 0.0;
    }
    let rows: usize = [CommitSide::Second, CommitSide::First]
        .into_iter()
        .map(|side| {
            let (listed, has_more, is_cut) = group(state, view, side);
            1 + listed + usize::from(has_more) + usize::from(is_cut)
        })
        .sum::<usize>()
        + 2;
    line * f32::from(u16::try_from(rows).unwrap_or(u16::MAX))
}

/// **Commits**: a group of the commits only in the second side, then one of those only in the first, each newest
/// first with its short hash, subject, author and how long ago, a click opening its page; a commit whose change the
/// other side has says **also in `<other>`**. A group lists 300 commits, then **Show all N commits…**; one with more
/// commits than were read says **The newest N are listed.** under its heading
///
/// Feature: compare.commits
pub fn commits(state: &CompareState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let CompareRead::Read(view) = state.read else {
        return intents;
    };
    let rect = ui.current_clip();
    let line = ui.theme().sizes.row_height;
    let mut column = Area::new(rect);
    let too_many = view.only_in_first + view.only_in_second > COMMIT_LIMIT;
    let heading = if too_many { words::TOO_MANY } else { words::COMMITS };
    let _ = ui.label(column.top(line), &Label::new(heading).kind(TextKind::Strong));
    let (first, second) = (&state.target.first.name, &state.target.second.name);
    let path = state.target.path.as_deref();
    for (side, name, other, count, whole) in [
        (
            CommitSide::Second,
            second,
            first,
            view.only_in_second,
            state.whole_counts.map(|(_, second)| second),
        ),
        (
            CommitSide::First,
            first,
            second,
            view.only_in_first,
            state.whole_counts.map(|(first, _)| first),
        ),
    ] {
        let read: Vec<&ComparedCommit> = view.commits.iter().filter(|commit| commit.side == side).collect();
        let also = read.iter().filter(|commit| commit.is_also_in_other_side).count();
        let heading = words::only_in(name, (count, whole), path, (also, other));
        let _ = ui.label(column.top(line), &Label::new(&heading).tone(Tone::Muted));
        let (listed, has_more, is_cut) = group(state, view, side);
        if is_cut {
            let note = words::newest_listed(read.len());
            let _ = ui.label(column.top(line), &Label::new(&note).tone(Tone::Muted));
        }
        for commit in read.iter().take(listed) {
            commit_row(state, (commit, other), ui, column.top(line), &mut intents);
        }
        if has_more {
            let label = words::show_all_commits(read.len());
            let button = Button::secondary(&label).kind(ButtonKind::Icon);
            let row = column.top(line);
            let (width, height) = ui.button_size(&button);
            let cell = place(row, (width.min(row.width), height), (Align::Start, Align::Center));
            if ui
                .button(("show all commits", side == CommitSide::First), cell, &button)
                .clicked
            {
                intents.push(Intent::Compare(CompareIntent::ShowAllCommits(state.id)));
            }
        }
    }
    intents
}

/// A commit only in one side: its short hash, subject and author, greyed out with **also in `<other>`** when the other
/// side has its change.
fn commit_row(
    state: &CompareState<'_>,
    (commit, other): (&ComparedCommit, &str),
    ui: &mut Ui<'_>,
    row: Rect,
    intents: &mut Vec<Intent>,
) {
    let when = crate::text::dates::relative_date(commit.author_date.0, state.now);
    let mut label = format!(
        "{}  {} \u{b7} {} \u{b7} {when}",
        commit.id.short(),
        commit.subject,
        commit.author_name
    );
    if commit.is_also_in_other_side {
        label.push_str(" \u{b7} ");
        label.push_str(&words::also_in(other));
    }
    let also_tooltip = words::also_in_tooltip(other);
    let mut button = Button::secondary(&label).kind(ButtonKind::Icon).with_icon(Icon::Commit);
    if commit.is_also_in_other_side {
        button = button.with_tooltip(&also_tooltip);
    }
    let (width, height) = ui.button_size(&button);
    let cell = place(row, (width.min(row.width), height), (Align::Start, Align::Center));
    if ui.button(("commit", commit.id.as_str()), cell, &button).clicked {
        intents.push(Intent::Compare(CompareIntent::OpenCommit(state.id, commit.id.clone())));
    }
}
