//! The island of the tabs, at the top of the window (`repo.tabs`): a tab for each repository open, the one shown
//! marked, each with its menu and a **×** on the one shown and the one under the pointer (`repo.close-tab`), and
//! **+**, which opens a repository of the workspace in a tab (`repo.open-tab`), a folder or a workspace file of VS
//! Code.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Icon, Key, Label, MenuEntry, MenuItem, Modifiers, NamedKey, Rect,
    Sense, TextKind, Tone, Ui,
};

use crate::graph::RepositoryEntry;
use crate::intent::Intent;
use crate::line::control_in;
use crate::text::header;
use crate::text::repo as words;

/// The widest a tab gets, in logical pixels.
pub const TAB_MOST: f32 = 200.0;

/// The key of the menu of **+**.
const OPEN_MENU: &str = "open menu";

/// What the island of the tabs shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsState<'a> {
    /// The tabs, in their order.
    pub tabs: &'a [TabEntry],
    /// The repositories of the workspace with no tab, which the menu of **+** lists (`repo.open-tab`).
    pub not_open: &'a [RepositoryEntry],
}

/// A tab, as the island draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabEntry {
    /// The name of its repository (`repo.name-repositories`).
    pub name: String,
    /// The folder of what it shows, for its tooltip.
    pub folder: String,
    /// Whether it is the tab shown.
    pub is_shown: bool,
    /// Whether an operation runs in its repository: it shows a spinner (`repo.switch-tab`).
    pub is_running: bool,
}

/// What an entry of the menu of **+** does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenCommand {
    /// Opens a repository with no tab, by its place among them.
    Repository(usize),
    /// **Open folder…**.
    Folder,
    /// **Open workspace file…**.
    WorkspaceFile,
}

/// What an entry of the menu of a tab does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabCommand {
    /// **Close tab**.
    Close,
    /// **Close other tabs**.
    CloseOthers,
}

/// The island of the tabs over the rectangle its caller clipped it to: each tab as wide as its name and the room of its
/// **×**, up to [`TAB_MOST`], a longer name cut at its start; the tab shown on the ground of a page with a border; a
/// **×** on it and on the tab under the pointer; **+** at the end. A middle click on a tab closes it, a right click opens its menu. `Ctrl+1` to `Ctrl+8` show the tab at that
/// place and `Ctrl+9` the last, `Ctrl+PageDown` and `Ctrl+PageUp` the next tab and the one before, going round.
///
/// Feature: repo.tabs
pub fn tabs_view(state: &TabsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let theme = *ui.theme();
    ui.fill_rect(rect, theme.colors.header_background);
    ui.region("tabs", (rect, AccessRole::Navigation, words::TABS), |ui| {
        let spacing = theme.sizes.spacing;
        let inner = inset(rect, Insets::symmetric(spacing.medium, 0.0));
        let mut area = Area::with_gap(inner, spacing.small);
        open_button(state.not_open, ui, &mut area, &mut intents);
        for (index, tab) in state.tabs.iter().enumerate() {
            let style = ui.text_style(TextKind::Medium);
            let padding = 2.0 * spacing.large;
            let close = theme.sizes.icon + 2.0 * spacing.small;
            let spinner = if tab.is_running {
                theme.sizes.icon + spacing.small
            } else {
                0.0
            };
            let width = (ui.measure(&tab.name, style) + padding + close + spinner).min(TAB_MOST);
            if area.rest().width < width {
                break;
            }
            let cell = control_in(ui, area.left(width), width);
            intents.extend(tab_button((index, tab), ui, (cell, padding, close)));
        }
    });
    intents.extend(keys(state.tabs, ui));
    intents
}

/// One tab, and what a click on it, on its **×**, a middle click or its menu asked.
fn tab_button(
    (index, tab): (usize, &TabEntry),
    ui: &mut Ui<'_>,
    (cell, padding, close): (Rect, f32, f32),
) -> Vec<Intent> {
    let theme = *ui.theme();
    let radius = theme.sizes.radius.medium;
    let id = ui.id(("tab", index, &tab.folder));
    let response = ui.interact(id, cell, Sense::CLICK);
    if tab.is_shown {
        ui.fill_rounded_rect(cell, radius, theme.colors.page_background);
        ui.border(
            cell,
            ui::Border {
                width: theme.sizes.border,
                radius,
                color: theme.colors.border,
            },
        );
    } else if response.hovered {
        ui.fill_rounded_rect(cell, radius, theme.colors.secondary_button_hover);
    }
    let mut parts = Area::new(inset(cell, Insets::symmetric(padding / 2.0, 0.0)));
    let close_cell = parts.right(close);
    if tab.is_running {
        running_spinner(ui, &mut parts, index);
    }
    let name_rect = parts.rest();
    let name = cut_at_start(ui, &tab.name, (name_rect.width, TextKind::Medium));
    let tone = if tab.is_shown { Tone::Normal } else { Tone::Muted };
    let _ = ui.label(name_rect, &Label::new(&name).kind(TextKind::Medium).tone(tone));
    let description = Description::new(AccessRole::Button, &tab.name).selected(tab.is_shown);
    ui.describe(id, cell, &description);
    ui.tooltip(&response, &tab.folder);
    let side = theme.sizes.icon + theme.sizes.spacing.small;
    let close_rect = place(close_cell, (side, side), (Align::End, Align::Center));
    let close_button = Button::icon(Icon::Close, words::CLOSE_TAB).kind(ButtonKind::Icon);
    // The room of the **×** is kept on every tab, so that a tab does not change its width under the pointer.
    let is_closed = (tab.is_shown || response.hovered)
        && ui
            .scope(("tab close", index), |ui| ui.button("close", close_rect, &close_button))
            .clicked;
    let entries = [
        MenuEntry::Item(MenuItem::new(words::CLOSE_TAB, TabCommand::Close)),
        MenuEntry::Item(MenuItem::new(words::CLOSE_OTHER_TABS, TabCommand::CloseOthers)),
    ];
    let mut intents = Vec::new();
    match ui.context_menu(("tab menu", index), &response, &entries) {
        Some(TabCommand::Close) => intents.push(Intent::CloseTab(index)),
        Some(TabCommand::CloseOthers) => intents.push(Intent::CloseOtherTabs(index)),
        None => {}
    }
    if is_closed || response.middle_clicked {
        intents.push(Intent::CloseTab(index));
    } else if response.clicked {
        intents.push(Intent::SwitchTab(index));
    }
    intents
}

/// The spinner of a tab where an operation runs, before its name.
fn running_spinner(ui: &mut Ui<'_>, parts: &mut Area, index: usize) {
    let theme = *ui.theme();
    let mark = parts.left(theme.sizes.icon);
    let _gap = parts.left(theme.sizes.spacing.small);
    ui.spinner(mark);
    let id = ui.id(("tab spinner", index));
    ui.describe(
        id,
        mark,
        &Description::new(AccessRole::Progress, words::OPERATION_RUNNING),
    );
}

/// A name that does not fit cut at its start with `…`, so that its last part, the repository's own folder, stays.
#[must_use]
pub fn cut_at_start(ui: &Ui<'_>, name: &str, (room, kind): (f32, TextKind)) -> String {
    let style = ui.text_style(kind);
    if ui.measure(name, style) <= room {
        return name.to_owned();
    }
    let mut starts = name.char_indices().map(|(at, _)| at).skip(1);
    loop {
        let Some(at) = starts.next() else {
            return "\u{2026}".to_owned();
        };
        let cut = format!("\u{2026}{}", name.get(at..).unwrap_or_default());
        if ui.measure(&cut, style) <= room {
            return cut;
        }
    }
}

/// **+** at the end of the island, with its menu: the repositories of the workspace with no tab, then **Open
/// folder…** and **Open workspace file…**.
fn open_button(not_open: &[RepositoryEntry], ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let side = ui.theme().sizes.control_height;
    let rect = control_in(ui, area.right(side), side);
    let button = Button::icon(Icon::Plus, words::OPEN_ANOTHER).kind(ButtonKind::Icon);
    let mut entries: Vec<MenuEntry<'_, OpenCommand>> = not_open
        .iter()
        .enumerate()
        .map(|(index, entry)| MenuEntry::Item(MenuItem::new(entry.name.as_str(), OpenCommand::Repository(index))))
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(
        MenuItem::new(header::OPEN_FOLDER, OpenCommand::Folder).with_icon(Icon::Folder),
    ));
    entries.push(MenuEntry::Item(MenuItem::new(
        words::OPEN_WORKSPACE_FILE,
        OpenCommand::WorkspaceFile,
    )));
    match ui.menu_button(OPEN_MENU, rect, &button, &entries) {
        Some(OpenCommand::Repository(index)) => {
            intents.extend(not_open.get(index).map(|entry| Intent::OpenTab(entry.folder.clone())));
        }
        Some(OpenCommand::Folder) => intents.push(Intent::OpenFolder),
        Some(OpenCommand::WorkspaceFile) => intents.push(Intent::OpenWorkspaceFile),
        None => {}
    }
}

/// The keys of the tabs, when nothing else took them.
fn keys(tabs: &[TabEntry], ui: &Ui<'_>) -> Vec<Intent> {
    let count = tabs.len();
    let shown = tabs.iter().position(|tab| tab.is_shown);
    if count < 2 {
        return Vec::new();
    }
    ui.unclaimed_keys()
        .into_iter()
        .filter(|key| key.modifiers == Modifiers::CONTROL)
        .filter_map(|key| match key.key {
            Key::Character(digit @ '1'..='8') => {
                let place = digit.to_digit(10).and_then(|place| usize::try_from(place).ok())?;
                (place <= count).then(|| Intent::SwitchTab(place - 1))
            }
            Key::Character('9') => Some(Intent::SwitchTab(count - 1)),
            Key::Named(NamedKey::PageDown) => Some(Intent::SwitchTab(shown.map_or(0, |shown| (shown + 1) % count))),
            Key::Named(NamedKey::PageUp) => {
                Some(Intent::SwitchTab(shown.map_or(0, |shown| (shown + count - 1) % count)))
            }
            _ => None,
        })
        .collect()
}
