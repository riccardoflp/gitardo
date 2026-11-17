//! The rows of **Repositories** (`repo.tabs`): a row for each repository open, and under it, while it is unfolded by
//! the chevron at its left, a row for each of its worktrees and its submodules, indented (`repo.group-tabs`). Each is
//! one line: its name, then what it is on (`repo.worktree-summary`), smaller, on its right; the one shown is marked,
//! and a click shows it.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, Rect, Response, Sense, TextKind, Tone, Ui};

use super::summary::{TabSummary, summary_line, summary_width};
use super::tabs::{TabEntry, TabMemberEntry, cut_at_start};
use crate::icons;
use crate::intent::Intent;
use crate::text::repo as words;

/// A row of **Repositories**: a repository open, by its place, or one of its worktrees or its submodules under it.
#[derive(Clone, Copy, Debug)]
pub(super) enum Row<'a> {
    /// The row of a repository open.
    Repository(usize, &'a TabEntry),
    /// The row of a worktree or a submodule of a repository open, under the row of that repository.
    Member(usize, &'a TabEntry, &'a TabMemberEntry),
}

/// The rows of the repositories open, each followed by the rows of its worktrees and its submodules while it is
/// unfolded.
pub(super) fn rows_of(tabs: &[TabEntry]) -> Vec<Row<'_>> {
    let mut rows = Vec::new();
    for (index, tab) in tabs.iter().enumerate() {
        rows.push(Row::Repository(index, tab));
        if tab.is_unfolded {
            let members = tab.members.get(1..).unwrap_or_default();
            rows.extend(members.iter().map(|member| Row::Member(index, tab, member)));
        }
    }
    rows
}

/// Whether a row of the repositories open has a chevron, and so each keeps its room at its left.
pub(super) fn has_chevrons(tabs: &[TabEntry]) -> bool {
    tabs.iter().any(|tab| !tab.members.is_empty())
}

/// What a row draws and how it is marked.
struct RowLook<'a> {
    /// Its key among the widgets of the frame.
    key: (&'static str, usize, &'a str),
    /// Its name.
    name: &'a str,
    /// What it is on, once known.
    summary: Option<&'a TabSummary>,
    /// What its second line says while its summary is not known.
    unknown: &'a str,
    /// Its icon.
    icon: ui::Icon,
    /// How far its icon is from the left of the row.
    indent: f32,
    /// Whether it is the one shown.
    is_shown: bool,
    /// Whether its repository is the one shown, which its name says in the normal tone.
    is_tab_shown: bool,
    /// Its tooltip: its folder.
    folder: &'a str,
}

/// The room at the left of a row for its chevron, when the rows have chevrons.
fn chevron_room(ui: &Ui<'_>, has_chevrons: bool) -> f32 {
    let theme = ui.theme();
    if has_chevrons {
        theme.sizes.icon + theme.sizes.spacing.tiny
    } else {
        0.0
    }
}

/// A row: its background, the bar of the one shown and its icon, and what a click, `Enter` or `Space` on it gave.
/// Returns the response and the rest of the row on the right of its icon.
fn draw_row(ui: &mut Ui<'_>, cell: Rect, look: &RowLook<'_>) -> (Response, Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(look.key);
    let response = ui.interact(id, cell, Sense::CLICK | Sense::FOCUS);
    if look.is_shown {
        ui.fill_rect(cell, theme.colors.list_inactive_selection);
        let bar = Rect::new(cell.x, cell.y, 2.0 * theme.sizes.border, cell.height);
        ui.fill_rect(bar, theme.colors.accent);
    } else if response.hovered {
        ui.fill_rect(cell, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(cell, theme.sizes.radius.small);
    }
    let inner = inset(cell, Insets::symmetric(spacing.medium, 0.0));
    let mut parts = Area::with_gap(inner, spacing.small);
    if look.indent > 0.0 {
        let _ = parts.left((look.indent - spacing.small).max(0.0));
    }
    ui.icon_in(parts.left(theme.sizes.icon), look.icon, Tone::Muted);
    let description = Description::new(AccessRole::Button, look.name).selected(look.is_shown);
    ui.describe(id, cell, &description);
    ui.tooltip(&response, look.folder);
    (response, parts.rest())
}

/// The name of a row and what it is on, on one line: the name on the left, what it is on on the right, in the small
/// text of details. When both do not fit, the name keeps at least half the room, cut at its start with `…`, and the
/// branch is cut with `…`.
fn name_and_summary(ui: &mut Ui<'_>, rect: Rect, look: &RowLook<'_>) {
    let gap = ui.theme().sizes.spacing.medium;
    let name_width = ui.measure(look.name, ui.text_style(TextKind::Medium));
    let said_width = match look.summary {
        Some(summary) => summary_width(ui, summary),
        None => ui.measure(look.unknown, ui.text_style(TextKind::Detail)),
    };
    let name_room = if said_width > 0.0 {
        name_width.min((rect.width - gap - said_width).max(rect.width * 0.5))
    } else {
        rect.width
    };
    let mut parts = Area::with_gap(rect, gap);
    draw_name(ui, parts.left(name_room), (look.name, look.is_tab_shown));
    let right = parts.rest();
    let width = said_width.min(right.width.max(0.0));
    let said = Rect::new(right.right() - width, right.y, width, right.height);
    match look.summary {
        Some(summary) => summary_line(ui, said, summary),
        None if !look.unknown.is_empty() => {
            let _ = ui.label(said, &Label::new(look.unknown).kind(TextKind::Detail).tone(Tone::Muted));
        }
        None => {}
    }
}

/// The name of a row over its room, cut at its start with `…` when it is too long.
fn draw_name(ui: &mut Ui<'_>, rect: Rect, (name, is_tab_shown): (&str, bool)) {
    let name = cut_at_start(ui, name, (rect.width, TextKind::Medium));
    let tone = if is_tab_shown { Tone::Normal } else { Tone::Muted };
    let _ = ui.label(rect, &Label::new(&name).kind(TextKind::Medium).tone(tone));
}

/// The row of a repository open: the chevron that folds the rows of its worktrees and its submodules, when it has
/// some; its name, which says the worktree or the submodule it shows after ` › ` while those rows are folded; a
/// spinner while an operation runs in it; and its **×** on the row shown and the row under the pointer. Unfolded, it
/// says its own folder and is marked while that is shown; a click shows its own folder.
pub(super) fn repository_row(
    (index, tab): (usize, &TabEntry),
    ui: &mut Ui<'_>,
    (cell, has_chevrons): (Rect, bool),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let side = theme.sizes.icon + theme.sizes.spacing.small;
    let own = tab.members.first().filter(|_| tab.is_unfolded);
    let look = RowLook {
        key: ("tab", index, &tab.folder),
        name: own.map_or(tab.name.as_str(), |_| tab.repository.as_str()),
        summary: own.map_or(tab.summary.as_ref(), |own| own.summary.as_ref()),
        unknown: "",
        icon: icons::WORKTREE,
        indent: chevron_room(ui, has_chevrons),
        is_shown: tab.is_shown && own.is_none_or(|own| own.is_shown),
        is_tab_shown: tab.is_shown,
        folder: &tab.folder,
    };
    let (response, line) = draw_row(ui, cell, &look);
    let mut parts = Area::with_gap(line, theme.sizes.spacing.small);
    let close_cell = parts.right(side);
    if tab.is_running {
        running_spinner(ui, parts.right(theme.sizes.icon), index);
    }
    name_and_summary(ui, parts.rest(), &look);
    let mut intents = Vec::new();
    if !tab.members.is_empty() {
        let width = theme.sizes.icon + theme.sizes.spacing.medium;
        intents.extend(chevron_button(
            (index, tab),
            ui,
            Rect::new(cell.x, cell.y, width, cell.height),
        ));
    }
    let close_rect = place(close_cell, (side, side), (Align::End, Align::Center));
    let close_button = Button::icon(icons::CLOSE, words::CLOSE_TAB).kind(ButtonKind::Icon);
    // The room of the **×** is kept on every row, so that the name does not move under the pointer.
    let is_closed = (tab.is_shown || response.hovered)
        && ui
            .scope(("tab close", index), |ui| ui.button("close", close_rect, &close_button))
            .clicked;
    intents.extend(super::tabs::row_menu((index, tab), ui, &response));
    if is_closed || response.middle_clicked {
        intents.push(Intent::CloseTab(index));
    } else if response.clicked {
        intents.push(match own {
            Some(own) if !own.is_shown => Intent::ShowInTab(index, own.folder.clone()),
            _ => Intent::SwitchTab(index),
        });
    }
    intents
}

/// The row of a worktree or a submodule under the row of its repository: indented, a submodule of a submodule further;
/// the name of the worktree's folder or the submodule's path, and what it is on, or the branch git listed for it while
/// it is not read yet. Marked while it is shown; a click shows it in the tab of its repository.
pub(super) fn member_row(
    (index, tab, member): (usize, &TabEntry, &TabMemberEntry),
    ui: &mut Ui<'_>,
    (cell, has_chevrons): (Rect, bool),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let step = theme.sizes.icon + theme.sizes.spacing.small;
    #[allow(clippy::cast_precision_loss, reason = "a submodule is inside a few others at most")]
    let depth = (member.depth + 1) as f32;
    let folder = member.folder.display().to_string();
    let icon = if member.is_submodule {
        icons::SUBMODULE
    } else {
        icons::WORKTREE
    };
    let look = RowLook {
        key: ("tab member", index, &folder),
        name: &member.name,
        summary: member.summary.as_ref(),
        unknown: if member.is_submodule { "" } else { &member.label },
        icon,
        indent: chevron_room(ui, has_chevrons) + depth * step,
        is_shown: tab.is_shown && member.is_shown,
        is_tab_shown: tab.is_shown && member.is_shown,
        folder: &folder,
    };
    let (response, line) = draw_row(ui, cell, &look);
    // The room of the **×** of a repository is kept, so that what each row is on ends at the same place.
    let mut parts = Area::with_gap(line, theme.sizes.spacing.small);
    let _ = parts.right(theme.sizes.icon + theme.sizes.spacing.small);
    name_and_summary(ui, parts.rest(), &look);
    if response.clicked && !look.is_shown {
        vec![Intent::ShowInTab(index, member.folder.clone())]
    } else {
        Vec::new()
    }
}

/// The chevron at the left of the row of a repository with worktrees or submodules, right while their rows are folded
/// and down while they are shown under it; a click, `Enter` or `Space` folds or unfolds them.
fn chevron_button((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let theme = *ui.theme();
    let id = ui.id(("tab chevron", index));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.hovered {
        ui.fill_rect(rect, theme.colors.list_hover);
    }
    if response.shows_focus {
        ui.focus_ring(rect, theme.sizes.radius.small);
    }
    let icon = if tab.is_unfolded {
        icons::UNFOLDED
    } else {
        icons::FOLDED
    };
    ui.icon_in(rect, icon, Tone::Muted);
    ui.describe(
        id,
        rect,
        &Description::new(AccessRole::Button, words::TAB_MEMBERS).expanded(tab.is_unfolded),
    );
    ui.tooltip(&response, words::TAB_MEMBERS);
    response.clicked.then_some(Intent::FoldTab(index))
}

/// The spinner of a row where an operation runs, before its **×**.
fn running_spinner(ui: &mut Ui<'_>, cell: Rect, index: usize) {
    let side = ui.theme().sizes.icon;
    let mark = place(cell, (side, side), (Align::Center, Align::Center));
    ui.spinner(mark);
    let id = ui.id(("tab spinner", index));
    ui.describe(
        id,
        mark,
        &Description::new(AccessRole::Progress, words::OPERATION_RUNNING),
    );
}
