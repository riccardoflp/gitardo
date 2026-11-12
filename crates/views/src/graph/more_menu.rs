//! The **…** menu at the end of the header (`graph.more-menu`), with only what is built: **Refresh**, and **Select
//! repository…** when the workspace has more than one repository.

use ui::{Button, ButtonKind, Icon, MenuEntry, MenuItem, Ui};

use crate::intent::Intent;
use crate::text::header as words;

/// The key of the menu.
const MORE_MENU: &str = "more menu";

/// What the **…** menu offers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoreMenuState {
    /// Whether a repository is shown, which **Refresh** reads again.
    pub can_refresh: bool,
    /// Whether **Select repository…** is offered: the workspace has more than one repository (`repo.select`).
    pub can_select_repository: bool,
}

/// What an item of the menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoreCommand {
    /// **Refresh**.
    Refresh,
    /// **Select repository…**.
    SelectRepository,
}

/// The **…** button and its menu, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it further: `graph.more-menu` (**Branch**, **Stash**, **Undo**, the switches of the
/// timeline, the ways to the pages), `repo.schedule-reads` (what **Refresh** asks for).
pub fn more_menu_view(state: &MoreMenuState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::icon(Icon::More, words::MORE).kind(ButtonKind::Outline);
    let refresh = MenuItem::new(words::REFRESH, MoreCommand::Refresh).with_icon(Icon::Refresh);
    let mut entries = vec![MenuEntry::Item(if state.can_refresh {
        refresh
    } else {
        refresh.disabled()
    })];
    if state.can_select_repository {
        entries.push(MenuEntry::Separator);
        entries.push(MenuEntry::Item(MenuItem::new(
            words::SELECT_REPOSITORY,
            MoreCommand::SelectRepository,
        )));
    }
    match ui.menu_button(MORE_MENU, rect, &button, &entries) {
        Some(MoreCommand::Refresh) => vec![Intent::Refresh],
        Some(MoreCommand::SelectRepository) => vec![Intent::PickRepository],
        None => Vec::new(),
    }
}
