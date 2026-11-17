//! **Repositories**, the first section of the side column (`repo.tabs`): a row for each repository open, the one
//! shown marked, what each is on under its name, each with its **▾** and a **×** on the row shown and the row under
//! the pointer (`repo.close-tab`); and **+** in the heading of the section, which opens a repository of the workspace in
//! a tab (`repo.open-tab`), a folder or a workspace file of VS Code.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Key, Label, MenuEntry, MenuItem, Modifiers, NamedKey, Rect, Sense,
    TextKind, Tone, Ui,
};

use super::summary::{TabSummary, summary_line};
use crate::graph::RepositoryEntry;
use crate::icons;
use crate::intent::Intent;
use crate::line::control_in;
use crate::number::float_of_count;
use crate::text::header;
use crate::text::repo as words;
use crate::text::worktree as worktree_words;

/// The key of the menu of **+**.
const OPEN_MENU: &str = "open menu";

/// What **Repositories** shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsState<'a> {
    /// The repositories open, in their order.
    pub tabs: &'a [TabEntry],
    /// The repositories of the workspace with no row, which the menu of **+** lists (`repo.open-tab`).
    pub not_open: &'a [RepositoryEntry],
}

/// A repository open, as its row in **Repositories** draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabEntry {
    /// The name of its repository (`repo.name-repositories`), with the worktree or the submodule it shows after ` › `.
    pub name: String,
    /// The name of its repository alone, as the header says it (`graph.where`).
    pub repository: String,
    /// The worktree or the submodule it shows when it is not the repository's own folder: the name of the worktree's
    /// folder, or the path of the submodule, as the header says it.
    pub place: Option<TabPlace>,
    /// The folder of what it shows, for its tooltip.
    pub folder: String,
    /// Whether it is the one shown.
    pub is_shown: bool,
    /// Whether an operation runs in its repository: it shows a spinner (`repo.switch-tab`).
    pub is_running: bool,
    /// What it shows is on, once it is known (`repo.worktree-summary`).
    pub summary: Option<TabSummary>,
    /// The repositories of its group its **▾** lists, its own first; none when it has no other worktree and no
    /// submodule, and then it has no **▾** (`repo.group-tabs`).
    pub members: Vec<TabMemberEntry>,
}

/// The worktree or the submodule a row shows, when it is not the repository's own folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabPlace {
    /// The name of the worktree's folder, or the path of the submodule.
    pub name: String,
    /// Whether it is a submodule, rather than a worktree.
    pub is_submodule: bool,
}

/// A repository of the group of a row, in its **▾**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabMemberEntry {
    /// Its repository's name, a worktree's branch or `detached at <short>`, or a submodule's path.
    pub label: String,
    /// Its folder.
    pub folder: std::path::PathBuf,
    /// How many submodules it is inside, for its indent.
    pub depth: usize,
    /// Whether the row shows it, which a check marks.
    pub is_shown: bool,
    /// What it is on, once it is known (`repo.worktree-summary`).
    pub summary: Option<TabSummary>,
}

/// What an entry of the menu of **+** does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenCommand {
    /// Opens a repository with no row, by its place among them.
    Repository(usize),
    /// **Open folder…**.
    Folder,
    /// **Open workspace file…**.
    WorkspaceFile,
}

/// What an entry of the menu of a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TabCommand {
    /// **Close tab**.
    Close,
    /// **Close other tabs**.
    CloseOthers,
    /// **Reveal in File Explorer**, or the name the system gives it.
    Reveal,
    /// **Copy path**.
    CopyPath,
}

/// The height of a row of **Repositories**: two lines, its name and what it is on.
#[must_use]
pub fn row_height(ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    2.0 * theme.sizes.row_height - theme.sizes.spacing.medium
}

/// The height the rows of **Repositories** want, all of them.
#[must_use]
pub fn rows_height(ui: &Ui<'_>, state: &TabsState<'_>) -> f32 {
    float_of_count(state.tabs.len()) * row_height(ui) + ui.theme().sizes.spacing.small
}

/// The rows of **Repositories** over the rectangle their caller clipped them to, scrolling when they do not fit: on
/// the first line of each the name of its repository, with the worktree or the submodule it shows after ` › `, cut at
/// its start when it is too long, a spinner while an operation runs in it, its **▾** and its **×**; on the second line
/// what it shows is on. The row shown has the background of a row selected and a bar in the color of the accent on its
/// left. A middle click on a row closes it, a right click opens its menu. `Ctrl+1` to `Ctrl+8` show the row at that
/// place and `Ctrl+9` the last, `Ctrl+PageDown` and `Ctrl+PageUp` the next and the one before, going round; on macOS
/// `Cmd+1` to `Cmd+9`, `Cmd+Shift+]` and `Cmd+Shift+[`.
///
/// Feature: repo.tabs
pub fn tabs_view(state: &TabsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let height = row_height(ui);
    let content = (rect.width, rows_height(ui, state));
    ui.region("tabs", (rect, AccessRole::Navigation, words::TABS), |ui| {
        let _ = ui.scroll_area("tabs rows", rect, content, |ui, inside| {
            let mut y = inside.y;
            for (index, tab) in state.tabs.iter().enumerate() {
                let row = Rect::new(inside.x, y, inside.width, height);
                y += height;
                if row.bottom() < rect.y || row.y > rect.bottom() {
                    continue;
                }
                intents.extend(tab_row((index, tab), ui, row));
            }
        });
    });
    intents.extend(keys(state.tabs, ui));
    intents
}

/// One row, and what a click on it, on its **×**, a middle click or its menu asked.
fn tab_row((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, cell: Rect) -> Vec<Intent> {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("tab", index, &tab.folder));
    let response = ui.interact(id, cell, Sense::CLICK);
    if tab.is_shown {
        ui.fill_rect(cell, theme.colors.list_inactive_selection);
        let bar = Rect::new(cell.x, cell.y, 2.0 * theme.sizes.border, cell.height);
        ui.fill_rect(bar, theme.colors.accent);
    } else if response.hovered {
        ui.fill_rect(cell, theme.colors.list_hover);
    }
    let inner = inset(cell, Insets::symmetric(spacing.medium, spacing.tiny));
    let mut lines = Area::new(inner);
    let first = lines.top(inner.height * 0.5);
    let second = lines.rest();
    let mut parts = Area::with_gap(first, spacing.small);
    ui.icon_in(parts.left(theme.sizes.icon), icons::WORKTREE, Tone::Muted);
    let side = theme.sizes.icon + spacing.small;
    let close_cell = parts.right(side);
    let mut intents = Vec::new();
    if !tab.members.is_empty() {
        let members_rect = parts.right(side);
        intents.extend(members_button((index, tab), ui, members_rect));
    }
    if tab.is_running {
        running_spinner(ui, parts.right(theme.sizes.icon), index);
    }
    let name_rect = parts.rest();
    let name = cut_at_start(ui, &tab.name, (name_rect.width, TextKind::Medium));
    let tone = if tab.is_shown { Tone::Normal } else { Tone::Muted };
    let _ = ui.label(name_rect, &Label::new(&name).kind(TextKind::Medium).tone(tone));
    let indent = theme.sizes.icon + spacing.small;
    let summary_rect = Rect::new(
        second.x + indent,
        second.y,
        (second.width - indent).max(0.0),
        second.height,
    );
    if let Some(summary) = &tab.summary {
        summary_line(ui, summary_rect, summary);
    }
    let description = Description::new(AccessRole::Button, &tab.name).selected(tab.is_shown);
    ui.describe(id, cell, &description);
    ui.tooltip(&response, &tab.folder);
    let close_rect = place(close_cell, (side, side), (Align::End, Align::Center));
    let close_button = Button::icon(icons::CLOSE, words::CLOSE_TAB).kind(ButtonKind::Icon);
    // The room of the **×** is kept on every row, so that the name does not move under the pointer.
    let is_closed = (tab.is_shown || response.hovered)
        && ui
            .scope(("tab close", index), |ui| ui.button("close", close_rect, &close_button))
            .clicked;
    intents.extend(row_menu((index, tab), ui, &response));
    if is_closed || response.middle_clicked {
        intents.push(Intent::CloseTab(index));
    } else if response.clicked {
        intents.push(Intent::SwitchTab(index));
    }
    intents
}

/// The context menu of a row: **Close tab**, **Close other tabs**, then **Reveal in File Explorer** on the row shown,
/// and **Copy path**, which copies the folder of what the row shows.
fn row_menu((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, response: &ui::Response) -> Option<Intent> {
    let mut entries = vec![
        MenuEntry::Item(MenuItem::new(words::CLOSE_TAB, TabCommand::Close)),
        MenuEntry::Item(MenuItem::new(words::CLOSE_OTHER_TABS, TabCommand::CloseOthers)),
        MenuEntry::Separator,
    ];
    if tab.is_shown {
        entries.push(MenuEntry::Item(MenuItem::new(
            worktree_words::reveal(),
            TabCommand::Reveal,
        )));
    }
    entries.push(MenuEntry::Item(MenuItem::new(
        worktree_words::COPY_PATH,
        TabCommand::CopyPath,
    )));
    match ui.context_menu(("tab menu", index), response, &entries)? {
        TabCommand::Close => Some(Intent::CloseTab(index)),
        TabCommand::CloseOthers => Some(Intent::CloseOtherTabs(index)),
        TabCommand::Reveal => Some(Intent::RevealTab(index)),
        TabCommand::CopyPath => {
            ui.copy_to_clipboard(tab.folder.clone());
            None
        }
    }
}

/// The **▾** of a row whose repository has other worktrees or submodules, and the one chosen in its menu: the
/// repository, its worktrees, then its submodules indented by how many submodules they are inside, the one the row
/// shows checked (`repo.group-tabs`).
fn members_button((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let labels: Vec<(String, String, String)> = tab
        .members
        .iter()
        .map(|member| {
            let label = format!("{}{}", "\u{2003}".repeat(member.depth), member.label);
            let counts = member
                .summary
                .as_ref()
                .map(super::summary::counts_words)
                .unwrap_or_default();
            (label, counts, member.folder.display().to_string())
        })
        .collect();
    let entries: Vec<MenuEntry<'_, usize>> = tab
        .members
        .iter()
        .zip(&labels)
        .enumerate()
        .map(|(at, (member, (label, counts, folder)))| {
            let mut item = MenuItem::new(label.as_str(), at).with_tooltip(folder.as_str());
            if !counts.is_empty() {
                item = item.with_shortcut(counts.as_str());
            }
            MenuEntry::Item(if member.is_shown {
                item.with_icon(icons::CHOSEN)
            } else {
                item
            })
        })
        .collect();
    let side = ui.theme().sizes.icon + ui.theme().sizes.spacing.small;
    let rect = place(rect, (side, side), (Align::Center, Align::Center));
    let button = Button::icon(icons::CHOOSE, words::TAB_MEMBERS).kind(ButtonKind::Icon);
    let chosen = ui.scope(("tab members", index), |ui| {
        ui.menu_button("members", rect, &button, &entries)
    })?;
    let member = tab.members.get(chosen)?;
    Some(Intent::ShowInTab(index, member.folder.clone()))
}

/// The spinner of a row where an operation runs, before its **▾** and its **×**.
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

/// **+** in the heading of **Repositories**, over a rectangle, with its menu: the repositories of the workspace with
/// no row, then **Open folder…** and **Open workspace file…**.
pub(crate) fn open_button(not_open: &[RepositoryEntry], ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let side = rect.height;
    let rect = control_in(ui, rect, side);
    let button = Button::icon(icons::ADD, words::OPEN_ANOTHER).kind(ButtonKind::Icon);
    let mut entries: Vec<MenuEntry<'_, OpenCommand>> = not_open
        .iter()
        .enumerate()
        .map(|(index, entry)| MenuEntry::Item(MenuItem::new(entry.name.as_str(), OpenCommand::Repository(index))))
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(
        MenuItem::new(header::OPEN_FOLDER, OpenCommand::Folder).with_icon(icons::FOLDER),
    ));
    entries.push(MenuEntry::Item(MenuItem::new(
        words::OPEN_WORKSPACE_FILE,
        OpenCommand::WorkspaceFile,
    )));
    match ui.menu_button(OPEN_MENU, rect, &button, &entries)? {
        OpenCommand::Repository(index) => not_open.get(index).map(|entry| Intent::OpenTab(entry.folder.clone())),
        OpenCommand::Folder => Some(Intent::OpenFolder),
        OpenCommand::WorkspaceFile => Some(Intent::OpenWorkspaceFile),
    }
}

/// The keys of the rows, when nothing else took them: `Ctrl` with a digit, `PageDown` or `PageUp`; on macOS `Cmd` with
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
