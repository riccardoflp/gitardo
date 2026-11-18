//! The menus of the changes: the context menu of the row of a file (`changes.row-menu`), which has what the buttons of
//! the row did, each with the icon of its action (`ui.menu-icons`), and **…** at the right of the heading of the list,
//! which has the buttons the width of the heading does not hold (`changes.pane`).

use ui::{Button, ButtonKind, MenuEntry, Rect, Response, Ui};

use super::actions::RowAction;
use crate::icons;
use crate::intent::Intent;
use crate::text::changes as words;

/// What the menu of the row of a file is made from.
#[derive(Clone, Debug, PartialEq)]
pub struct RowMenuState<'a> {
    /// The path of the file, which tells its menu from those of the other rows.
    pub path: &'a str,
    /// What can be done to the file, in the order of the menu.
    pub actions: &'a [RowAction],
    /// Whether an operation runs: the actions that change something cannot be chosen.
    pub is_running: bool,
    /// What happened to the row in this frame: the button of context menus released on it, or the key of context
    /// menus pressed while it has the keyboard, opens the menu, at the pointer or under the row.
    pub row: Response,
}

/// The context menu of the row of a file of the changes: opened by the right button on the row or by the menu key
/// while the row has the keyboard, drawn while it is open, with an item for each action, a line above the first of
/// each group; gives what the item chosen asks.
///
/// Feature: changes.row-menu
pub fn row_menu(state: &RowMenuState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let entries = entries_of(state.actions, state.is_running, |_| true);
    let chosen = ui.context_menu(("changes row menu", state.path), &state.row, &entries);
    asked(state.actions, chosen)
}

/// **…** at the right of the heading of the list, over a rectangle, with its menu: the actions on every file whose
/// buttons the heading has no room for, those at the places given being on the heading; gives what the item chosen
/// asks.
pub(super) fn more_menu(
    ui: &mut Ui<'_>,
    rect: Rect,
    (actions, on_heading, is_running): (&[RowAction], &[usize], bool),
) -> Vec<Intent> {
    let entries = entries_of(actions, is_running, |index| !on_heading.contains(&index));
    let button = Button::icon(icons::MORE, words::MORE_FOR_ALL).kind(ButtonKind::Icon);
    let chosen = ui.menu_button("changes heading menu", rect, &button, &entries);
    asked(actions, chosen)
}

/// The entries of a menu of actions, each carrying its place among them: those the menu lists, a line above the first
/// of each group but the first, and an item that cannot be chosen now drawn so.
fn entries_of(
    actions: &[RowAction],
    is_running: bool,
    is_listed: impl Fn(usize) -> bool,
) -> Vec<MenuEntry<'static, usize>> {
    let mut entries = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        if !is_listed(index) {
            continue;
        }
        if action.is_group_start && !entries.is_empty() {
            entries.push(MenuEntry::Separator);
        }
        let item = icons::action(action.icon, action.words, index);
        entries.push(MenuEntry::Item(if action.is_offered(is_running) {
            item
        } else {
            item.disabled()
        }));
    }
    entries
}

/// What the action at a place asks, when one was chosen.
fn asked(actions: &[RowAction], chosen: Option<usize>) -> Vec<Intent> {
    chosen
        .and_then(|index| actions.get(index))
        .and_then(|action| action.intent.clone())
        .into_iter()
        .collect()
}
