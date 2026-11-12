//! How each row of the branch rail is drawn and what it does: the title of a list opens and closes it and opens its
//! menu, and **Show all `<n>`…** lists a list in full. The entries are in [`super::entry`].

use std::time::Duration;

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Label, MenuAnchor, Rect, Sense, TextKind, Theme, Tone, Ui,
};

use super::ReferencesState;
use super::entry::entry_row;
use super::menu::{ListCommand, list_menu};
use super::rows::{Entry, ListHead, RailRow};
use crate::graph::HoverTarget;
use crate::intent::{Intent, RailIntent, RailList};
use crate::text::rail as words;

/// The height of the title of **Kept aside**.
const TITLE: f32 = 20.0;
/// The height of the title of a list that is not kept aside.
const HEADING: f32 = 24.0;
/// The height of the rows of the local branches.
const BRANCH_ROW: f32 = 32.0;
/// The height of the rows of the remote branches and of the lists of **Kept aside**.
const QUIET_ROW: f32 = 30.0;
/// The side of the dot of a branch.
pub(super) const DOT: f32 = 10.0;

/// What the rows of the rail asked for in a frame, and the reference the pointer rests on.
#[derive(Debug, Default)]
pub(super) struct RailReport {
    /// What was asked.
    pub(super) intents: Vec<Intent>,
    /// The reference under the pointer, with since when it rests there, for its history.
    pub(super) hover: Option<(HoverTarget, Duration)>,
}

/// What the rows are drawn from.
#[derive(Clone, Copy)]
pub(super) struct Context<'s, 'a> {
    /// The rail.
    pub(super) state: &'s ReferencesState<'a>,
    /// Whether it shows only icons.
    pub(super) is_icons: bool,
}

impl RailRow<'_> {
    /// The height of the row, in logical pixels.
    pub(super) fn height(&self, theme: &Theme) -> f32 {
        match self {
            RailRow::KeptTitle => TITLE,
            RailRow::Heading(_) => HEADING,
            RailRow::Kept(_) => QUIET_ROW,
            RailRow::Entry { entry, .. } => match entry {
                Entry::Branch(_) => BRANCH_ROW,
                Entry::Remote(_) => QUIET_ROW,
                _ => theme.sizes.row_height,
            },
            RailRow::InFull { .. } => theme.sizes.row_height,
            RailRow::Gap => theme.sizes.spacing.large + theme.sizes.spacing.small,
        }
    }
}

/// Draws a row of the rail, and notes what it asked for.
pub(super) fn draw_row(
    ui: &mut Ui<'_>,
    context: Context<'_, '_>,
    row: &RailRow<'_>,
    rect: Rect,
    report: &mut RailReport,
) {
    match row {
        RailRow::KeptTitle => title(ui, rect, words::KEPT_ASIDE),
        RailRow::Heading(head) | RailRow::Kept(head) => {
            if context.is_icons {
                kept_icon(ui, rect, head);
            } else {
                report.intents.extend(list_title(ui, context, head, rect));
            }
        }
        RailRow::Entry { list, entry, marks } => entry_row(ui, context, (list, entry, *marks), rect, report),
        RailRow::InFull { list, count } => {
            let text = words::show_all_of(*count);
            let button = Button::secondary(&text).kind(ButtonKind::Icon);
            let (width, height) = ui.button_size(&button);
            let indent = indent(ui.theme());
            let cell = place(
                inset(
                    rect,
                    Insets {
                        left: indent,
                        ..Insets::default()
                    },
                ),
                (width, height),
                (Align::Start, Align::Center),
            );
            if ui.button(("in full", list), cell, &button).clicked {
                report.intents.push(Intent::Rail(RailIntent::InFull(list.clone())));
            }
        }
        RailRow::Gap => {}
    }
}

/// How far the entries of a list are from its edge: past the chevron of its title.
pub(super) fn indent(theme: &Theme) -> f32 {
    theme.sizes.spacing.medium + theme.sizes.icon + theme.sizes.spacing.small
}

/// The title of **Kept aside**.
fn title(ui: &mut Ui<'_>, rect: Rect, text: &str) {
    let padded = inset(rect, Insets::symmetric(ui.theme().sizes.spacing.medium, 0.0));
    let _ = ui.label(padded, &Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted));
}

/// The name of a list.
pub(super) fn list_name(list: &RailList) -> String {
    match list {
        RailList::Favorites => words::FAVORITES.to_owned(),
        RailList::Recent => words::RECENT.to_owned(),
        RailList::Local => words::LOCAL.to_owned(),
        RailList::Remote(remote) => words::remote_title(remote),
        RailList::Stashes => words::STASHES.to_owned(),
        RailList::Worktrees => words::WORKTREES.to_owned(),
        RailList::Tags => words::TAGS.to_owned(),
        RailList::Submodules => words::SUBMODULES.to_owned(),
    }
}

/// The icon of a list.
fn list_icon(list: &RailList) -> Icon {
    match list {
        RailList::Favorites => Icon::Star,
        RailList::Recent => Icon::Clock,
        RailList::Local => Icon::Branch,
        RailList::Remote(_) => Icon::Cloud,
        RailList::Stashes => Icon::Stash,
        RailList::Worktrees => Icon::Worktree,
        RailList::Tags => Icon::Tag,
        RailList::Submodules => Icon::Folder,
    }
}

/// The count of a list: how many it lists, or `<matches> of <all>` while something is typed and it is not empty.
fn count_words(head: &ListHead) -> String {
    if head.is_typed && head.all > 0 {
        words::matches_of(head.matches, head.all)
    } else {
        head.all.to_string()
    }
}

/// The icon of a list of **Kept aside**, in the narrow layout.
fn kept_icon(ui: &mut Ui<'_>, rect: Rect, head: &ListHead) {
    let side = ui.theme().sizes.icon;
    let cell = place(rect, (side, side), (Align::Center, Align::Center));
    ui.icon_in(cell, list_icon(&head.list), Tone::Muted);
    let id = ui.id(("kept icon", &head.list));
    let name = list_name(&head.list);
    let count = head.all.to_string();
    ui.describe(id, cell, &Description::new(AccessRole::Label, &name).value(&count));
}

/// The title of a list: its chevron, its icon, its name and its count, and **…** under the pointer when it has a
/// menu. A click opens or closes it; a right click or **…** opens its menu.
fn list_title(ui: &mut Ui<'_>, context: Context<'_, '_>, head: &ListHead, rect: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let is_kept = head.list.is_kept_aside();
    let id = ui.id(("list", &head.list));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    let menu_key = ("list menu", &head.list);
    let entries = list_menu(&head.list, context.state.sort);
    let is_menu_open = ui.is_menu_open(menu_key);
    if response.hovered || is_menu_open {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.medium);
    }
    let mut parts = Area::with_gap(inset(rect, Insets::symmetric(spacing.medium, 0.0)), spacing.small);
    let chevron = if head.is_open {
        Icon::ChevronDown
    } else {
        Icon::ChevronRight
    };
    ui.icon_in(parts.left(theme.sizes.icon), chevron, Tone::Muted);
    ui.icon_in(parts.left(theme.sizes.icon), list_icon(&head.list), Tone::Muted);
    if !entries.is_empty() && (response.hovered || is_menu_open) {
        let button = Button::icon(Icon::More, words::LIST_MENU).kind(ButtonKind::Icon);
        let side = rect.height;
        if ui
            .button(("list menu button", &head.list), parts.right(side), &button)
            .clicked
        {
            ui.open_menu(menu_key, MenuAnchor::Below(rect));
        }
    }
    let count = count_words(head);
    let (kind, tone) = if is_kept {
        (TextKind::Body, Tone::Normal)
    } else {
        (TextKind::SmallStrong, Tone::Muted)
    };
    let count_width = ui.measure(&count, ui.text_style(kind));
    let _ = ui.label(
        parts.right(count_width),
        &Label::new(&count).kind(kind).tone(Tone::Muted),
    );
    let name = list_name(&head.list);
    let _ = ui.label(parts.rest(), &Label::new(&name).kind(kind).tone(tone));
    let description = Description::new(AccessRole::Button, &name)
        .value(&count)
        .expanded(head.is_open);
    ui.describe(id, rect, &description);
    if response.secondary_clicked && !entries.is_empty() {
        ui.open_menu(menu_key, MenuAnchor::Pointer);
    }
    let mut intents = Vec::new();
    if response.clicked {
        intents.push(Intent::Rail(RailIntent::Toggle(head.list.clone())));
    }
    if let Some(ListCommand::Sort(order)) = ui.menu(menu_key, &entries) {
        intents.push(Intent::Rail(RailIntent::Sort(order)));
    }
    intents
}
