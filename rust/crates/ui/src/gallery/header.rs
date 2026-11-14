//! The header of the window: the repository with its menu, the branch checked out, Quick search, what runs or what was
//! done last, Sync with its menu, and the button of the Activity drawer.

use render::{Icon, Point, Rect, TextRun};

use super::menus;
use super::{Field, GalleryIntent, GalleryState, ICON_RAIL_BELOW, Overlay};
use crate::access::{AccessRole, Description};
use crate::frame::{TextKind, Ui};
use crate::interaction::Sense;
use crate::layout::{Align, Area, Insets, inset, place};
use crate::widget::Tone;
use crate::widget::button::{Button, ButtonKind};
use crate::widget::input_box::InputBox;
use crate::widget::label::Label;
use crate::widget::metrics;
use crate::widget::panel::Axis;

/// The key of the menu of the repository.
const REPOSITORY: &str = "repository";
/// The key of the menu of Sync.
const SYNC: &str = "sync";
/// The widest Quick search is.
const SEARCH_WIDTH: f32 = 420.0;
/// The branch checked out.
const BRANCH: &str = "feature/login-page";
/// How far the branch checked out is from main.
const DISTANCE: &str = "2 ahead \u{b7} 12 behind main";

/// Declares the header over its strip at the top of the window.
pub(super) fn header(state: &GalleryState, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.header_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    ui.region("header region", (rect, AccessRole::Header, "Repository"), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0));
        let mut area = Area::with_gap(inner, spacing.large);
        repository(ui, &mut area, intents);
        branch_chip(ui, &mut area, intents);
        activity_button(state, ui, &mut area, intents);
        sync(ui, &mut area, intents);
        what_runs(state, ui, &mut area);
        quick_search(state, ui, area.rest(), intents);
    });
}

/// A control of the height of the others, in the middle of a strip of the header, down.
fn control_in(ui: &Ui<'_>, cell: Rect, width: f32) -> Rect {
    let height = metrics::control_height(ui.theme());
    place(cell, (width, height), (Align::Start, Align::Center))
}

/// The pill of the repository, which opens its menu.
fn repository(ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<GalleryIntent>) {
    let accent = ui.theme().colors.accent;
    let button = Button::secondary("gitardo")
        .kind(ButtonKind::Pill)
        .with_dot(accent)
        .with_trailing(Icon::ChevronDown)
        .with_tooltip("Repositories, theme and settings");
    let (width, _) = ui.button_size(&button);
    let rect = control_in(ui, area.left(width), width);
    let themes = menus::theme_menu();
    let rail = menus::rail_menu();
    let entries = menus::repository_menu((&themes, &rail));
    if let Some(command) = ui.menu_button(REPOSITORY, rect, &button, &entries) {
        menus::run_command(ui, command, None, intents);
    }
}

/// The branch checked out and how far it is from main, which opens the picker of branches.
fn branch_chip(ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let is_wide = ui.window().width >= ICON_RAIL_BELOW;
    let name = ui.measure(BRANCH, ui.text_style(TextKind::Medium));
    let distance = if is_wide {
        ui.measure(DISTANCE, ui.text_style(TextKind::Detail)) + spacing.medium
    } else {
        0.0
    };
    let width = theme.sizes.icon + spacing.medium + name + distance + 2.0 * spacing.medium;
    let rect = control_in(ui, area.left(width), width);
    let id = ui.id("branch chip");
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.large, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.large);
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.medium);
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Branch, Tone::Lane(1));
    let _ = ui.draw_label(parts.left(name), &Label::new(BRANCH).kind(TextKind::Medium));
    if is_wide {
        let words = Label::new(DISTANCE).kind(TextKind::Detail).tone(Tone::Muted);
        let _ = ui.draw_label(parts.rest(), &words);
    }
    let description = Description::new(AccessRole::Button, BRANCH).value(DISTANCE);
    ui.describe(id, rect, &description);
    ui.tooltip(&response, "Switch to another branch");
    if response.clicked {
        intents.push(GalleryIntent::Open(Overlay::PickBranch));
    }
}

/// Quick search, in the middle of what is left: a click or a key in it opens the picker of commits, branches, files
/// and commands.
fn quick_search(state: &GalleryState, ui: &mut Ui<'_>, cell: Rect, intents: &mut Vec<GalleryIntent>) {
    let width = cell.width.min(SEARCH_WIDTH);
    if width < ui.theme().sizes.icon * 4.0 {
        return;
    }
    let height = metrics::control_height(ui.theme());
    let rect = place(cell, (width, height), (Align::Center, Align::Center));
    let input = InputBox::new(&state.search, "Commits, branches, files, commands")
        .with_icon(Icon::Search)
        .with_key_hint("Ctrl K");
    let answer = ui.input_box("quick search", rect, &input);
    if let Some(text) = answer.changed_text {
        intents.push(GalleryIntent::Typed(Field::Search, text));
        intents.push(GalleryIntent::Open(Overlay::QuickSearch));
    } else if answer.response.clicked || answer.is_submitted {
        intents.push(GalleryIntent::Open(Overlay::QuickSearch));
    }
}

/// What runs, with a spinner, or what was done last; only the spinner in a narrow window.
fn what_runs(state: &GalleryState, ui: &mut Ui<'_>, area: &mut Area) {
    let theme = *ui.theme();
    let is_wide = ui.window().width >= ICON_RAIL_BELOW;
    let style = ui.text_style(TextKind::Detail);
    let words = state.status.as_deref().unwrap_or("Fetching origin");
    let words_width = if is_wide {
        ui.measure(words, style).min(theme.sizes.rail_width) + theme.sizes.spacing.medium
    } else {
        0.0
    };
    let mut parts = Area::with_gap(area.right(theme.sizes.icon + words_width), theme.sizes.spacing.medium);
    let mark = parts.left(theme.sizes.icon);
    if state.status.is_some() {
        ui.icon_in(mark, Icon::Check, Tone::Success);
    } else {
        ui.spinner(mark);
    }
    if !is_wide {
        return;
    }
    let text = parts.rest();
    let line = place(text, (text.width, ui.line_height(style)), (Align::Start, Align::Center));
    let run = TextRun {
        text: words,
        style,
        color: theme.colors.muted_foreground,
        origin: Point::new(line.x, line.y),
        max_width: Some(text.width),
    };
    let _ = ui.text(&run);
    let id = ui.id("what runs");
    ui.describe(id, line, &Description::new(AccessRole::Status, words));
}

/// Sync, with how many commits it brings and sends, which opens its menu.
fn sync(ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<GalleryIntent>) {
    let button = Button::primary("\u{2193}2  \u{2191}1   Sync").with_tooltip("Fetch, pull and push");
    let (width, _) = ui.button_size(&button);
    let rect = control_in(ui, area.right(width), width);
    let entries = menus::sync_menu();
    if let Some(command) = ui.menu_button(SYNC, rect, &button, &entries) {
        menus::run_command(ui, command, None, intents);
    }
}

/// The button that opens and closes the Activity drawer.
fn activity_button(state: &GalleryState, ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<GalleryIntent>) {
    let side = metrics::control_height(ui.theme());
    let rect = control_in(ui, area.right(side), side);
    let button = Button::icon(Icon::Info, "Activity and command log").kind(ButtonKind::Outline);
    let response = ui.button("activity", rect, &button);
    ui.access_expanded(response.id, state.is_activity_open);
    if response.clicked {
        intents.push(if state.is_activity_open {
            GalleryIntent::CloseActivity
        } else {
            GalleryIntent::OpenActivity
        });
    }
}
