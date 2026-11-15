//! The Welcome page (`welcome.page`): **Open folder…** and the recent workspaces, then five steps that show where the
//! timeline, the branch rail and the inspector are and what to try first.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, Description, Icon, Label, Rect, TextKind, Tone, Ui};

use crate::intent::Intent;
use crate::text::welcome::{self as words, StepWords};

/// The widest the column of the page is.
const COLUMN_WIDTH: f32 = 720.0;
/// The height of the card of a step.
const STEP_HEIGHT: f32 = 144.0;
/// The side of the picture of a step.
const PICTURE: f32 = 64.0;
/// How many steps there are.
pub const STEP_COUNT: usize = 5;

/// What the Welcome page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeState<'a> {
    /// The recent workspaces, the latest first, each by its folders.
    pub recent: &'a [String],
    /// Which steps are done, in their order; a step stays done.
    pub done: [bool; STEP_COUNT],
    /// Whether there is a timeline to show: the window shows a workspace.
    pub has_timeline: bool,
}

/// The Welcome page, over the rectangle its caller clipped it to: its title, **Open folder…**, the recent workspaces
/// that open on a click, and the five steps, each ticked once done.
///
/// Feature cards that fill it further: the buttons of the steps that narrow the timeline, open a commit, a comparison,
/// a file and a worktree, and the pictures of the steps, which are icons for now, with `filter.show-only`,
/// `commit-page.open`, `compare.choose`, `blame.toggle-file` and `worktree.create`.
///
/// Feature: welcome.page
pub fn page_view(state: &WelcomeState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let width = COLUMN_WIDTH.min((rect.width - 2.0 * spacing.huge).max(0.0));
    let line = theme.sizes.row_height;
    let recent_height = if state.recent.is_empty() {
        0.0
    } else {
        line * (1.0 + count(state.recent.len())) + spacing.large
    };
    let steps_height = count(STEP_COUNT) * (STEP_HEIGHT + spacing.large);
    let height = 2.0 * spacing.huge + line * 2.0 + theme.sizes.control_height + recent_height + steps_height;
    let column_area = Rect::new(rect.x + (rect.width - width) * 0.5, rect.y, width, rect.height);
    ui.region("welcome", (rect, AccessRole::Main, words::TITLE), |ui| {
        let _ = ui.scroll_area("welcome page", column_area, (width, height), |ui, content| {
            let mut column = Area::with_gap(inset(content, Insets::symmetric(0.0, spacing.huge)), spacing.large);
            let _ = ui.heading(column.top(line * 2.0), words::TITLE);
            let open = Button::primary(words::OPEN_FOLDER).with_icon(Icon::Folder);
            let (open_width, open_height) = ui.button_size(&open);
            let cell = place(
                column.top(open_height),
                (open_width, open_height),
                (Align::Start, Align::Center),
            );
            if ui.button("open folder", cell, &open).clicked {
                intents.push(Intent::OpenFolder);
            }
            let file = Button::secondary(crate::text::repo::OPEN_WORKSPACE_FILE);
            let (file_width, file_height) = ui.button_size(&file);
            let beside = Rect::new(cell.right() + spacing.medium, cell.y, file_width, file_height);
            if ui.button("open workspace file", beside, &file).clicked {
                intents.push(Intent::OpenWorkspaceFile);
            }
            if !state.recent.is_empty() {
                recent(state.recent, ui, &mut column, &mut intents);
            }
            for (index, step) in words::STEPS.iter().enumerate() {
                let is_done = state.done.get(index).copied().unwrap_or(false);
                let card = column.top(STEP_HEIGHT);
                step_card(ui, (index, step, is_done), (card, state.has_timeline), &mut intents);
            }
        });
    });
    intents
}

/// A number of things, as a length to multiply.
fn count(things: usize) -> f32 {
    f32::from(u16::try_from(things).unwrap_or(u16::MAX))
}

/// The recent workspaces, the latest first, each opening on a click.
fn recent(recent: &[String], ui: &mut Ui<'_>, column: &mut Area, intents: &mut Vec<Intent>) {
    let line = ui.theme().sizes.row_height;
    let _ = ui.label(column.top(line), &Label::new(words::RECENT).kind(TextKind::Strong));
    for (index, label) in recent.iter().enumerate() {
        let row = column.top(line);
        let button = Button::secondary(label)
            .kind(ui::ButtonKind::Icon)
            .with_icon(Icon::Folder);
        let (width, height) = ui.button_size(&button);
        let cell = place(row, (width.min(row.width), height), (Align::Start, Align::Center));
        if ui.button(("recent", index), cell, &button).clicked {
            intents.push(Intent::OpenRecent(index));
        }
    }
}

/// The picture of a step: for now the icon of the place it is about.
fn picture(index: usize) -> Icon {
    match index {
        0 => Icon::Commit,
        1 => Icon::Branch,
        2 => Icon::Search,
        3 => Icon::File,
        _ => Icon::Worktree,
    }
}

/// The card of a step: its picture, its title with a tick once done, what it says, and its buttons. Only **Show the
/// timeline** does something yet, while there is a timeline to show; the others come with their cards.
fn step_card(
    ui: &mut Ui<'_>,
    (index, step, is_done): (usize, &StepWords, bool),
    (rect, has_timeline): (Rect, bool),
    intents: &mut Vec<Intent>,
) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.card(rect, theme.sizes.radius.large);
    let mut parts = Area::with_gap(inset(rect, Insets::all(spacing.large)), spacing.large);
    let tile = parts.left(PICTURE);
    let tile = place(tile, (PICTURE, PICTURE), (Align::Start, Align::Start));
    ui.fill_rounded_rect(tile, theme.sizes.radius.medium, theme.colors.page_background);
    ui.icon_in(tile, picture(index), Tone::Lane(index));
    let mut text = Area::with_gap(parts.rest(), spacing.small);
    let mut title_line = Area::with_gap(text.top(theme.sizes.row_height), spacing.small);
    if is_done {
        ui.icon_in(title_line.left(theme.sizes.icon), Icon::Check, Tone::Success);
    }
    let _ = ui.label(title_line.rest(), &Label::new(step.title).kind(TextKind::Strong));
    let id = ui.id(("step", index));
    let state_words = if is_done { words::DONE } else { "" };
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Label, step.title).value(state_words),
    );
    let buttons = text.bottom(theme.sizes.control_height);
    let _ = ui.paragraph(text.rest(), step.says, (TextKind::Detail, Tone::Muted));
    let mut row = Area::with_gap(buttons, spacing.medium);
    for (button_index, label) in step.buttons.iter().enumerate() {
        let works = index == 0 && has_timeline;
        let button = if works {
            Button::outline(label)
        } else {
            Button::outline(label).disabled()
        };
        let (width, _) = ui.button_size(&button);
        if ui
            .button(("step button", index, button_index), row.left(width), &button)
            .clicked
            && works
        {
            intents.push(Intent::ShowTimeline);
        }
    }
}
