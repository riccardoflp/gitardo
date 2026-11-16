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

/// Below this width of the window, in logical pixels, the island is one button.
pub const ONE_BUTTON_BELOW: f32 = 480.0;

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
    /// The repositories of its group its **▾** lists, its own first; none when it has no other worktree and no
    /// submodule, and then it has no **▾** (`repo.group-tabs`).
    pub members: Vec<TabMemberEntry>,
}

/// A repository of the group of a tab, in its **▾**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabMemberEntry {
    /// Its repository's name, a worktree's branch or `detached at <short>`, or a submodule's path.
    pub label: String,
    /// Its folder.
    pub folder: std::path::PathBuf,
    /// How many submodules it is inside, for its indent.
    pub depth: usize,
    /// Whether the tab shows it, which a check marks.
    pub is_shown: bool,
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

/// What an entry of the island drawn as one button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListCommand {
    /// Shows the tab at a place.
    Tab(usize),
    /// **Open a repository…**.
    Open,
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
/// **×** and its **▾**, up to [`TAB_MOST`], a longer name cut at its start; the tab shown on the ground of a page with
/// a border; a **×** on it and on the tab under the pointer; **+** at the end. Tabs that do not fit get narrower, down
/// to [`super::TAB_LEAST`], and those that still do not are listed in **… N**, never the tab shown. Below
/// [`ONE_BUTTON_BELOW`] the island is one button, the name of the tab shown with a chevron, listing every tab and
/// **Open a repository…**. A middle click on a tab closes it, a right click opens its menu. `Ctrl+1` to `Ctrl+8` show
/// the tab at that place and `Ctrl+9` the last, `Ctrl+PageDown` and `Ctrl+PageUp` the next tab and the one before,
/// going round; on macOS `Cmd+1` to `Cmd+9`, `Cmd+Shift+]` and `Cmd+Shift+[`.
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
        if ui.window().width < ONE_BUTTON_BELOW {
            intents.extend(one_button(state.tabs, ui, area.rest()));
            return;
        }
        open_button(state.not_open, ui, &mut area, &mut intents);
        let widths: Vec<f32> = state.tabs.iter().map(|tab| rooms(ui, tab).0).collect();
        let more = Button::secondary("").kind(ButtonKind::Icon);
        let label = words::more_tabs(state.tabs.len());
        let more_width = ui.button_size(&Button { label: &label, ..more }).0;
        let shown = state.tabs.iter().position(|tab| tab.is_shown);
        let fit = super::fit_tabs(&widths, shown, (area.rest().width, spacing.small, more_width));
        for (index, width) in fit.drawn {
            let Some(tab) = state.tabs.get(index) else {
                continue;
            };
            let cell = control_in(ui, area.left(width), width);
            let (_, padding, close) = rooms(ui, tab);
            intents.extend(tab_button((index, tab), ui, (cell, padding, close)));
        }
        if !fit.listed.is_empty() {
            let rect = area.left(more_width);
            intents.extend(more_button((state.tabs, &fit.listed), ui, rect));
        }
    });
    intents.extend(keys(state.tabs, ui));
    intents
}

/// The width a tab wants, as wide as its name up to [`TAB_MOST`]; the room on each side of its name; and the room of
/// its **×** and its **▾**.
fn rooms(ui: &Ui<'_>, tab: &TabEntry) -> (f32, f32, f32) {
    let theme = ui.theme();
    let spacing = theme.sizes.spacing;
    let padding = 2.0 * spacing.large;
    let mut close = theme.sizes.icon + 2.0 * spacing.small;
    if !tab.members.is_empty() {
        close += theme.sizes.icon + spacing.small;
    }
    let style = ui.text_style(TextKind::Medium);
    let width = (ui.measure(&tab.name, style) + padding + close).min(TAB_MOST);
    (width, padding, close)
}

/// **… N**, which lists the tabs that do not fit, in their order; choosing one shows it.
fn more_button((tabs, listed): (&[TabEntry], &[usize]), ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let label = words::more_tabs(listed.len());
    let button = Button::secondary(&label)
        .kind(ButtonKind::Icon)
        .with_tooltip(words::MORE_TABS);
    let rect = control_in(ui, rect, rect.width);
    let entries: Vec<MenuEntry<'_, usize>> = listed
        .iter()
        .filter_map(|index| {
            tabs.get(*index)
                .map(|tab| MenuEntry::Item(MenuItem::new(tab.name.as_str(), *index)))
        })
        .collect();
    ui.menu_button("more tabs", rect, &button, &entries)
        .map(Intent::SwitchTab)
}

/// The island as one button, in a narrow window: the name of the tab shown with a chevron, whose list has every tab,
/// the one shown checked, and **Open a repository…**.
fn one_button(tabs: &[TabEntry], ui: &mut Ui<'_>, room: Rect) -> Option<Intent> {
    let name = tabs
        .iter()
        .find(|tab| tab.is_shown)
        .map_or(words::OPEN_A_REPOSITORY, |tab| tab.name.as_str());
    let button = Button::secondary(name)
        .kind(ButtonKind::Icon)
        .with_trailing(Icon::ChevronDown);
    let width = ui.button_size(&button).0.min(room.width);
    let rect = control_in(ui, room, width);
    let mut entries: Vec<MenuEntry<'_, ListCommand>> = tabs
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let item = MenuItem::new(tab.name.as_str(), ListCommand::Tab(index));
            MenuEntry::Item(if tab.is_shown {
                item.with_icon(Icon::Check)
            } else {
                item
            })
        })
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(MenuItem::new(
        words::OPEN_A_REPOSITORY,
        ListCommand::Open,
    )));
    match ui.menu_button("tabs list", rect, &button, &entries)? {
        ListCommand::Tab(index) => Some(Intent::SwitchTab(index)),
        ListCommand::Open => Some(Intent::PickTabToOpen),
    }
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
    let room = cell.width - padding - close;
    let name = cut_at_start(ui, &tab.name, (room, TextKind::Medium));
    let tone = if tab.is_shown { Tone::Normal } else { Tone::Muted };
    let mut parts = Area::new(inset(cell, Insets::symmetric(padding / 2.0, 0.0)));
    let close_cell = parts.right(close);
    let _ = ui.label(parts.rest(), &Label::new(&name).kind(TextKind::Medium).tone(tone));
    let description = Description::new(AccessRole::Button, &tab.name).selected(tab.is_shown);
    ui.describe(id, cell, &description);
    ui.tooltip(&response, &tab.folder);
    let side = theme.sizes.icon + theme.sizes.spacing.small;
    let close_rect = place(close_cell, (side, side), (Align::End, Align::Center));
    let mut intents = Vec::new();
    if !tab.members.is_empty() {
        let members_rect = place(close_cell, (side, side), (Align::Start, Align::Center));
        intents.extend(members_button((index, tab), ui, members_rect));
    }
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

/// The **▾** of a tab whose repository has other worktrees or submodules, and the one chosen in its menu: the
/// repository, its worktrees, then its submodules indented by how many submodules they are inside, the one the tab
/// shows checked (`repo.group-tabs`).
fn members_button((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let labels: Vec<String> = tab
        .members
        .iter()
        .map(|member| format!("{}{}", "\u{2003}".repeat(member.depth), member.label))
        .collect();
    let entries: Vec<MenuEntry<'_, usize>> = tab
        .members
        .iter()
        .zip(&labels)
        .enumerate()
        .map(|(at, (member, label))| {
            let item = MenuItem::new(label.as_str(), at);
            MenuEntry::Item(if member.is_shown {
                item.with_icon(Icon::Check)
            } else {
                item
            })
        })
        .collect();
    let button = Button::icon(Icon::ChevronDown, words::TAB_MEMBERS).kind(ButtonKind::Icon);
    let chosen = ui.scope(("tab members", index), |ui| {
        ui.menu_button("members", rect, &button, &entries)
    })?;
    let member = tab.members.get(chosen)?;
    Some(Intent::ShowInTab(index, member.folder.clone()))
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

/// The keys of the tabs, when nothing else took them: `Ctrl` with a digit, `PageDown` or `PageUp`; on macOS `Cmd` with
/// a digit, `Cmd+Shift+]` or `Cmd+Shift+[`.
fn keys(tabs: &[TabEntry], ui: &Ui<'_>) -> Vec<Intent> {
    let count = tabs.len();
    let shown = tabs.iter().position(|tab| tab.is_shown);
    if count < 2 {
        return Vec::new();
    }
    let next = Intent::SwitchTab(shown.map_or(0, |shown| (shown + 1) % count));
    let before = Intent::SwitchTab(shown.map_or(0, |shown| (shown + count - 1) % count));
    let is_mac = cfg!(target_os = "macos");
    let digit_with = if is_mac { Modifiers::META } else { Modifiers::CONTROL };
    ui.unclaimed_keys()
        .into_iter()
        .filter_map(|key| match key.key {
            Key::Character(digit @ '1'..='8') if key.modifiers == digit_with => {
                let place = digit.to_digit(10).and_then(|place| usize::try_from(place).ok())?;
                (place <= count).then(|| Intent::SwitchTab(place - 1))
            }
            Key::Character('9') if key.modifiers == digit_with => Some(Intent::SwitchTab(count - 1)),
            Key::Named(NamedKey::PageDown) if !is_mac && key.modifiers == Modifiers::CONTROL => Some(next.clone()),
            Key::Named(NamedKey::PageUp) if !is_mac && key.modifiers == Modifiers::CONTROL => Some(before.clone()),
            Key::Character(']' | '}') if is_mac && key.modifiers == Modifiers::META | Modifiers::SHIFT => {
                Some(next.clone())
            }
            Key::Character('[' | '{') if is_mac && key.modifiers == Modifiers::META | Modifiers::SHIFT => {
                Some(before.clone())
            }
            _ => None,
        })
        .collect()
}
