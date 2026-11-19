//! **Repositories**, the first section of the side column (`repo.tabs`): a row for each repository open, the one
//! shown marked, what each is on beside its name, the rows of its worktrees and its submodules under it while it is
//! unfolded (`repo.group-tabs`), and a **×** on the row shown and the row under the pointer (`repo.close-tab`); and **+** in the heading of the section, which opens a repository of the workspace in
//! a tab (`repo.open-tab`), a folder or a workspace file of VS Code.

use ui::{AccessRole, Button, ButtonKind, Key, MenuEntry, Modifiers, NamedKey, Rect, TextKind, Ui};

use super::rows::{Row, has_chevrons, member_row, repository_row, rows_of};
use super::summary::TabSummary;
use crate::graph::RepositoryEntry;
use crate::icons;
use crate::intent::Intent;
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
    /// The repositories of its group, its own first, then its worktrees and its submodules, which are rows under its
    /// row while it is unfolded; none when it has no other worktree and no submodule, and then it has no chevron
    /// (`repo.group-tabs`).
    pub members: Vec<TabMemberEntry>,
    /// Whether the rows of its worktrees and its submodules are shown under its row.
    pub is_unfolded: bool,
}

/// The worktree or the submodule a row shows, when it is not the repository's own folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabPlace {
    /// The name of the worktree's folder, or the path of the submodule.
    pub name: String,
    /// Whether it is a submodule, rather than a worktree.
    pub is_submodule: bool,
}

/// A repository of the group of a row, as a row under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabMemberEntry {
    /// Its repository's name, a worktree's branch or `detached at <short>`, or a submodule's path, as the menu of the
    /// header lists it (`graph.where`).
    pub label: String,
    /// The name of its row: its repository's name, the name of a worktree's folder, or a submodule's path.
    pub name: String,
    /// Whether it is a submodule, rather than its repository or a worktree.
    pub is_submodule: bool,
    /// Its folder.
    pub folder: std::path::PathBuf,
    /// How many submodules it is inside, for its indent.
    pub depth: usize,
    /// Whether the row shows it, which a check marks.
    pub is_shown: bool,
    /// What it is on, once it is known (`repo.worktree-summary`).
    pub summary: Option<TabSummary>,
    /// The tooltip of its lock, **Locked: `<reason>`**, while it is a locked worktree (`worktree.locked`).
    pub lock: Option<String>,
}

/// What an entry of the menu of **+** does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpenCommand {
    /// Opens a repository with no row, by its place among them.
    Repository(usize),
    /// **Open folder…**.
    Folder,
    /// **Clone a repository…**.
    Clone,
    /// **Create a repository…**.
    Create,
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

/// The height of a row of **Repositories**: one line, its name and what it is on.
#[must_use]
pub fn row_height(ui: &Ui<'_>) -> f32 {
    ui.theme().sizes.row_height
}

/// The height the rows of **Repositories** want, all of them, those of the worktrees and the submodules unfolded
/// included.
#[must_use]
pub fn rows_height(ui: &Ui<'_>, state: &TabsState<'_>) -> f32 {
    float_of_count(rows_of(state.tabs).len()) * row_height(ui) + ui.theme().sizes.spacing.tiny
}

/// The rows of **Repositories** over the rectangle their caller clipped them to, scrolling when they do not fit: on
/// one line each the chevron that folds the rows of its worktrees and its submodules, the name of its repository,
/// with the worktree or the submodule it shows after ` › ` while those are folded, cut at its start when it is too long,
/// what it shows is on, smaller, on its right, a spinner while an operation runs in it and its **×**. Under it, while
/// it is unfolded, a row for each of its worktrees and its submodules, indented. The row shown has the background of a row selected and a bar in the color of the accent on its
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
        let has_chevrons = has_chevrons(state.tabs);
        let _ = ui.scroll_area("tabs rows", rect, content, |ui, inside| {
            let mut y = inside.y;
            for row in rows_of(state.tabs) {
                let cell = Rect::new(inside.x, y, inside.width, height);
                y += height;
                if cell.bottom() < rect.y || cell.y > rect.bottom() {
                    continue;
                }
                intents.extend(match row {
                    Row::Repository(index, tab) => repository_row((index, tab), ui, (cell, has_chevrons)),
                    Row::Member(index, tab, member) => member_row((index, tab, member), ui, (cell, has_chevrons)),
                });
            }
        });
    });
    intents.extend(keys(state.tabs, ui));
    intents
}

/// The context menu of a row: **Close tab**, **Close other tabs**, then **Reveal in File Explorer** on the row shown,
/// and **Copy path**, which copies the folder of what the row shows.
pub(super) fn row_menu((index, tab): (usize, &TabEntry), ui: &mut Ui<'_>, response: &ui::Response) -> Option<Intent> {
    let mut entries = vec![
        MenuEntry::Item(icons::action(icons::CLOSE, words::CLOSE_TAB, TabCommand::Close)),
        MenuEntry::Item(icons::action(
            icons::CLOSE,
            words::CLOSE_OTHER_TABS,
            TabCommand::CloseOthers,
        )),
        MenuEntry::Separator,
    ];
    if tab.is_shown {
        entries.push(MenuEntry::Item(icons::action(
            icons::REVEAL,
            worktree_words::reveal(),
            TabCommand::Reveal,
        )));
    }
    entries.push(MenuEntry::Item(icons::action(
        icons::COPY,
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
/// no row, then **Open folder…**, **Clone a repository…**, **Create a repository…** and **Open workspace file…**.
pub(crate) fn open_button(not_open: &[RepositoryEntry], ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    let side = rect.height;
    let rect = ui::layout::place(
        rect,
        (side, side),
        (ui::layout::Align::Center, ui::layout::Align::Center),
    );
    let button = Button::icon(icons::ADD, words::OPEN_ANOTHER).kind(ButtonKind::Icon);
    let mut entries: Vec<MenuEntry<'_, OpenCommand>> = not_open
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            MenuEntry::Item(icons::action(
                icons::WORKTREE,
                entry.name.as_str(),
                OpenCommand::Repository(index),
            ))
        })
        .collect();
    if !entries.is_empty() {
        entries.push(MenuEntry::Separator);
    }
    entries.push(MenuEntry::Item(icons::action(
        icons::FOLDER,
        header::OPEN_FOLDER,
        OpenCommand::Folder,
    )));
    entries.push(MenuEntry::Item(icons::operation_item(
        words::CLONE,
        "repo.clone",
        OpenCommand::Clone,
    )));
    entries.push(MenuEntry::Item(icons::operation_item(
        words::CREATE,
        "repo.init",
        OpenCommand::Create,
    )));
    entries.push(MenuEntry::Item(icons::action(
        icons::OPEN_FILE,
        words::OPEN_WORKSPACE_FILE,
        OpenCommand::WorkspaceFile,
    )));
    match ui.menu_button(OPEN_MENU, rect, &button, &entries)? {
        OpenCommand::Repository(index) => not_open.get(index).map(|entry| Intent::OpenTab(entry.folder.clone())),
        OpenCommand::Folder => Some(Intent::OpenFolder),
        OpenCommand::Clone => Some(new_repository("repo.clone")),
        OpenCommand::Create => Some(new_repository("repo.init")),
        OpenCommand::WorkspaceFile => Some(Intent::OpenWorkspaceFile),
    }
}

/// The intent that runs an operation that makes a repository (`repo.clone`, `repo.init`).
pub(crate) fn new_repository(feature: &'static str) -> Intent {
    Intent::Feature(crate::feature::FeatureIntent::Run(
        model::contract::FeatureId(feature),
        model::contract::Target::Head,
    ))
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
