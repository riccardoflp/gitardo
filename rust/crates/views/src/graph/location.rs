//! Where one is, at the start of the header (`graph.where`): the repository shown, the worktree or the submodule shown
//! when it is not the repository's own folder, and the branch checked out, as `gitardo › login › feature/login-page`.

use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, MenuEntry, Rect, Sense, TextKind, Tone, Ui};

use super::toolbar::HeadName;
use crate::icons;
use crate::intent::Intent;
use crate::line::control_in;
use crate::repo::{TabEntry, TabPlace, cut_at_start};
use crate::text::header as words;

/// What the start of the header says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhereState<'a> {
    /// The repositories open, the one shown among them, whose list the repository opens.
    pub tabs: &'a [TabEntry],
    /// What is checked out in what is shown, once it is read.
    pub head: Option<HeadName<'a>>,
    /// The folder of what is shown, for the tooltip of the branch.
    pub folder: &'a str,
}

/// The parts of the header's start, each a width that is cut before the others when the room is short.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WhereWidths {
    /// The repository.
    pub repository: f32,
    /// The worktree or the submodule, when there is one.
    pub place: f32,
    /// The branch.
    pub branch: f32,
}

/// The widths of the parts in a room: each as wide as it wants while all fit; otherwise the repository is cut first,
/// down to `least`, then the worktree, then the branch.
#[must_use]
pub fn fit_where(wanted: WhereWidths, (room, least): (f32, f32)) -> WhereWidths {
    let mut excess = wanted.repository + wanted.place + wanted.branch - room;
    let mut cut = |width: f32| {
        if excess <= 0.0 {
            return width;
        }
        let given = (width - least.min(width)).min(excess);
        excess -= given;
        width - given
    };
    let repository = cut(wanted.repository);
    let place = cut(wanted.place);
    WhereWidths {
        repository,
        place,
        branch: (wanted.branch - excess.max(0.0)).max(0.0),
    }
}

/// The start of the header over the rectangle its caller clipped it to: the repository shown, which lists the
/// repositories open to show another; ` › ` and the worktree or the submodule shown, with its icon, which opens the
/// **▾** of its repository's row, when it is not the repository's own folder; ` › ` and the branch checked out, or
/// `detached at 3f2a1c9`, or `no commits yet`, with the folder in its tooltip and without how far it is from its
/// upstream, which **Sync** says. Gives the width it took.
///
/// Feature: graph.where
pub fn where_view(state: &WhereState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let Some((index, tab)) = state.tabs.iter().enumerate().find(|(_, tab)| tab.is_shown) else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let separator = words::WHERE_SEPARATOR;
    let separator_width = ui.measure(separator, ui.text_style(TextKind::Body));
    let head = state.head.map(head_words);
    let place_button = tab.place.as_ref().map(place_button);
    let wanted = WhereWidths {
        repository: ui.button_size(&repository_button(&tab.repository)).0,
        place: place_button
            .as_ref()
            .map_or(0.0, |button| ui.button_size(button).0 + separator_width),
        branch: head.as_ref().map_or(0.0, |head| {
            separator_width + theme.sizes.icon + spacing.small + ui.measure(head, ui.text_style(TextKind::Medium))
        }),
    };
    let widths = fit_where(wanted, (rect.width, 3.0 * theme.sizes.icon));
    let mut parts = Area::new(rect);
    let mut intents = Vec::new();
    let room = parts.left(widths.repository);
    intents.extend(repositories_menu((state.tabs, &tab.repository), ui, room));
    if let Some(place) = &tab.place {
        separator_label(ui, parts.left(separator_width));
        let room = parts.left((widths.place - separator_width).max(0.0));
        intents.extend(members_menu((index, tab, place), ui, room));
    }
    if let Some(head) = head {
        separator_label(ui, parts.left(separator_width));
        branch_label(
            (&head, state.folder),
            ui,
            parts.left((widths.branch - separator_width).max(0.0)),
        );
    }
    intents
}

/// The words of what is checked out.
fn head_words(head: HeadName<'_>) -> String {
    match head {
        HeadName::Branch(name) => name.to_owned(),
        HeadName::Detached(short) => words::detached_at(short),
        HeadName::NoCommits => words::NO_COMMITS_YET.to_owned(),
    }
}

/// The button of the repository.
fn repository_button(name: &str) -> Button<'_> {
    Button::secondary(name)
        .kind(ButtonKind::Icon)
        .with_tooltip(words::REPOSITORIES_OPEN)
}

/// The button of the worktree or the submodule shown.
fn place_button(place: &TabPlace) -> Button<'_> {
    let icon = if place.is_submodule {
        icons::SUBMODULE
    } else {
        icons::WORKTREE
    };
    Button::secondary(&place.name)
        .kind(ButtonKind::Icon)
        .with_icon(icon)
        .with_tooltip(crate::text::repo::TAB_MEMBERS)
}

/// ` › ` between two parts.
fn separator_label(ui: &mut Ui<'_>, rect: Rect) {
    let _ = ui.label(
        rect,
        &Label::new(words::WHERE_SEPARATOR)
            .tone(Tone::Muted)
            .align(Align::Center),
    );
}

/// The repository, cut at its start to its room, which lists the repositories open; choosing one shows it.
fn repositories_menu((tabs, name): (&[TabEntry], &str), ui: &mut Ui<'_>, room: Rect) -> Option<Intent> {
    let full = ui.button_size(&repository_button(name)).0;
    let padding = full - ui.measure(name, ui.text_style(TextKind::Body));
    let cut = cut_at_start(ui, name, ((room.width - padding).max(0.0), TextKind::Body));
    let button = repository_button(&cut);
    let rect = control_in(ui, room, room.width);
    let entries: Vec<MenuEntry<'_, usize>> = tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let item = icons::choice(tab.name.as_str(), index, tab.is_shown).with_tooltip(tab.folder.as_str());
            MenuEntry::Item(item)
        })
        .collect();
    ui.menu_button("where repositories", rect, &button, &entries)
        .map(Intent::SwitchTab)
}

/// The worktree or the submodule shown, cut at its start to its room, which opens the **▾** of the row of its
/// repository; choosing another shows it in the row.
fn members_menu((index, tab, place): (usize, &TabEntry, &TabPlace), ui: &mut Ui<'_>, room: Rect) -> Option<Intent> {
    let full = ui.button_size(&place_button(place)).0;
    let padding = full - ui.measure(&place.name, ui.text_style(TextKind::Body));
    let cut = TabPlace {
        name: cut_at_start(ui, &place.name, ((room.width - padding).max(0.0), TextKind::Body)),
        is_submodule: place.is_submodule,
    };
    let button = place_button(&cut);
    let rect = control_in(ui, room, room.width);
    let entries: Vec<MenuEntry<'_, usize>> = tab
        .members
        .iter()
        .enumerate()
        .map(|(at, member)| MenuEntry::Item(icons::choice(member.label.as_str(), at, member.is_shown)))
        .collect();
    let chosen = ui.menu_button("where members", rect, &button, &entries)?;
    let member = tab.members.get(chosen)?;
    Some(Intent::ShowInTab(index, member.folder.clone()))
}

/// The branch checked out, with its icon, cut at its start to its room, and the folder in its tooltip.
fn branch_label((head, folder): (&str, &str), ui: &mut Ui<'_>, room: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let rect = control_in(ui, room, room.width);
    let id = ui.id("where branch");
    let response = ui.interact(id, rect, Sense::HOVER);
    let mut parts = Area::with_gap(rect, spacing.small);
    let icon = parts.left(theme.sizes.icon);
    ui.icon_in(
        place(icon, (icon.width, icon.width), (Align::Start, Align::Center)),
        icons::BRANCH,
        Tone::Lane(0),
    );
    let name_rect = parts.rest();
    let name = cut_at_start(ui, head, (name_rect.width, TextKind::Medium));
    let _ = ui.label(name_rect, &Label::new(&name).kind(TextKind::Medium));
    ui.describe(id, rect, &Description::new(AccessRole::Label, head));
    ui.tooltip(&response, folder);
}
