//! The menus of the gallery — of the repository in the header, of Sync, of a commit in the timeline and in the
//! inspector, and of a branch in the rail — what their items do, and the shortcuts of the window.

use render::{Icon, ThemeName};

use super::{GalleryIntent, GalleryState, INSPECTOR_OVER_BELOW, Overlay, RailMode, data};
use crate::frame::Ui;
use crate::input::{Key, Modifiers, NamedKey};
use crate::widget::menu::{MenuEntry, MenuItem};

/// What the items of the menus of the gallery do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    /// Opens an overlay.
    Open(Overlay),
    /// Opens the find bar.
    Find,
    /// Changes the theme.
    Theme(ThemeName),
    /// Shows the branch rail in a way.
    Rail(RailMode),
    /// Opens another repository, by its name.
    Repository(&'static str),
    /// Copies the hash of the commit the menu was opened on.
    CopyHash,
    /// Cherry-picks the commit.
    CherryPick,
    /// Resets the branch to the commit, in a way.
    Reset(&'static str),
    /// Reverts the commit.
    Revert,
    /// Fetches from origin.
    Fetch,
    /// Pushes the branch checked out.
    Push,
    /// Pulls and pushes the branch checked out.
    Sync,
    /// Switches to the branch of the rail the menu was opened on.
    SwitchBranch,
    /// Signs in to GitHub.
    SignIn,
}

/// The key of the context menu of the commits of the timeline.
pub(super) const ROW_MENU: &str = "row menu";
/// The key of the find bar.
pub(super) const FIND: &str = "find";

/// The entries of the submenu of themes.
pub(super) fn theme_menu() -> [MenuEntry<'static, MenuCommand>; 5] {
    [
        MenuEntry::Item(MenuItem::new("System", MenuCommand::Theme(ThemeName::System))),
        MenuEntry::Item(MenuItem::new("Dark", MenuCommand::Theme(ThemeName::Dark))),
        MenuEntry::Item(MenuItem::new("Light", MenuCommand::Theme(ThemeName::Light))),
        MenuEntry::Item(MenuItem::new(
            "High contrast dark",
            MenuCommand::Theme(ThemeName::HighContrastDark),
        )),
        MenuEntry::Item(MenuItem::new(
            "High contrast light",
            MenuCommand::Theme(ThemeName::HighContrastLight),
        )),
    ]
}

/// The entries of the submenu of the ways to show the branch rail.
pub(super) fn rail_menu() -> [MenuEntry<'static, MenuCommand>; 3] {
    [
        MenuEntry::Item(MenuItem::new("With the names", MenuCommand::Rail(RailMode::Full))),
        MenuEntry::Item(MenuItem::new("Icons only", MenuCommand::Rail(RailMode::Icons))),
        MenuEntry::Item(MenuItem::new("Hidden", MenuCommand::Rail(RailMode::Hidden))),
    ]
}

/// The entries of the menu of the repository, in the header, with its submenus of themes and of the rail.
pub(super) fn repository_menu<'menu>(
    (themes, rail): (
        &'menu [MenuEntry<'menu, MenuCommand>; 5],
        &'menu [MenuEntry<'menu, MenuCommand>; 3],
    ),
) -> [MenuEntry<'menu, MenuCommand>; 14] {
    let item = |label, command| MenuEntry::Item(MenuItem::new(label, command));
    [
        MenuEntry::Item(MenuItem::new("gitardo", MenuCommand::Repository("gitardo")).with_icon(Icon::Check)),
        item("gitardo-site", MenuCommand::Repository("gitardo-site")),
        MenuEntry::Item(
            MenuItem::new("Open a repository", MenuCommand::Repository("another repository")).with_icon(Icon::Folder),
        ),
        MenuEntry::Separator,
        MenuEntry::Item(
            MenuItem::new("Create a branch", MenuCommand::Open(Overlay::CreateBranch)).with_icon(Icon::Branch),
        ),
        item("Delete the branch", MenuCommand::Open(Overlay::ConfirmDelete)),
        MenuEntry::Item(
            MenuItem::new("Switch to a branch", MenuCommand::Open(Overlay::PickBranch)).with_shortcut("Ctrl+P"),
        ),
        MenuEntry::Item(
            MenuItem::new("Choose branches", MenuCommand::Open(Overlay::PickBranches)).with_shortcut("Ctrl+Shift+P"),
        ),
        MenuEntry::Item(
            MenuItem::new("Find in page", MenuCommand::Find)
                .with_icon(Icon::Search)
                .with_shortcut("Ctrl+F"),
        ),
        MenuEntry::Separator,
        MenuEntry::Submenu {
            label: "Theme",
            icon: Some(Icon::Gear),
            entries: themes,
        },
        MenuEntry::Submenu {
            label: "Branch rail",
            icon: None,
            entries: rail,
        },
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new("Sign in to GitHub", MenuCommand::SignIn).disabled()),
    ]
}

/// The entries of the menu of Sync.
pub(super) fn sync_menu() -> [MenuEntry<'static, MenuCommand>; 5] {
    [
        MenuEntry::Item(MenuItem::new("Fetch", MenuCommand::Fetch).with_icon(Icon::ArrowDown)),
        MenuEntry::Item(MenuItem::new("Pull", MenuCommand::Open(Overlay::Pull))),
        MenuEntry::Item(MenuItem::new("Push", MenuCommand::Push).with_icon(Icon::ArrowUp)),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new("Sync with origin", MenuCommand::Sync).with_icon(Icon::Refresh)),
    ]
}

/// The entries of the submenu of the ways to reset.
pub(super) fn reset_menu() -> [MenuEntry<'static, MenuCommand>; 3] {
    [
        MenuEntry::Item(MenuItem::new("Soft", MenuCommand::Reset("soft"))),
        MenuEntry::Item(MenuItem::new("Mixed", MenuCommand::Reset("mixed"))),
        MenuEntry::Item(MenuItem::new("Hard", MenuCommand::Reset("hard"))),
    ]
}

/// The place of the submenu of the ways to reset in the context menu of the commits.
pub(super) const RESET_ENTRY: usize = 4;

/// The entries of the context menu of the commits of the timeline, and of the menu of the inspector.
pub(super) fn row_menu<'menu>(resets: &'menu [MenuEntry<'menu, MenuCommand>; 3]) -> [MenuEntry<'menu, MenuCommand>; 7] {
    [
        MenuEntry::Item(MenuItem::new("Copy the hash", MenuCommand::CopyHash).with_shortcut("Ctrl+C")),
        MenuEntry::Item(
            MenuItem::new("Create a branch here", MenuCommand::Open(Overlay::CreateBranch)).with_icon(Icon::Branch),
        ),
        MenuEntry::Item(MenuItem::new("Cherry-pick", MenuCommand::CherryPick).with_icon(Icon::Commit)),
        MenuEntry::Separator,
        MenuEntry::Submenu {
            label: "Reset the branch to here",
            icon: None,
            entries: resets,
        },
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new("Revert", MenuCommand::Revert).disabled()),
    ]
}

/// The entries of the context menu of the branches of the rail.
pub(super) fn branch_menu() -> [MenuEntry<'static, MenuCommand>; 5] {
    [
        MenuEntry::Item(MenuItem::new("Switch to this branch", MenuCommand::SwitchBranch).with_icon(Icon::Check)),
        MenuEntry::Item(
            MenuItem::new("Create a branch from here", MenuCommand::Open(Overlay::CreateBranch))
                .with_icon(Icon::Branch),
        ),
        MenuEntry::Item(MenuItem::new(
            "Choose the branches to show",
            MenuCommand::Open(Overlay::PickBranches),
        )),
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new(
            "Delete the branch",
            MenuCommand::Open(Overlay::ConfirmDelete),
        )),
    ]
}

/// Does what a command chosen in a menu asks; the row is the commit, or the branch of the rail, the menu was opened on.
pub(super) fn run_command(ui: &mut Ui<'_>, command: MenuCommand, row: Option<usize>, intents: &mut Vec<GalleryIntent>) {
    let hash = row.map(data::short_hash).unwrap_or_default();
    let intent = match command {
        MenuCommand::Open(overlay) => GalleryIntent::Open(overlay),
        MenuCommand::Find => {
            ui.open_find(FIND);
            return;
        }
        MenuCommand::SignIn => return,
        MenuCommand::Theme(theme) => GalleryIntent::ChoseTheme(theme),
        MenuCommand::Rail(mode) => GalleryIntent::ShowRail(mode),
        MenuCommand::Repository(name) => GalleryIntent::Did(format!("Opened {name}")),
        MenuCommand::CopyHash => {
            ui.copy_to_clipboard(hash.clone());
            GalleryIntent::Did(format!("Copied {hash}"))
        }
        MenuCommand::CherryPick => GalleryIntent::Did(format!("Cherry-picked {hash}")),
        MenuCommand::Reset(mode) => GalleryIntent::Did(format!("Reset main to {hash}, {mode}")),
        MenuCommand::Revert => GalleryIntent::Did(format!("Reverted {hash}")),
        MenuCommand::Fetch => GalleryIntent::Did(String::from("Fetched origin")),
        MenuCommand::Push => GalleryIntent::Did(String::from("Pushed feature/login-page")),
        MenuCommand::Sync => GalleryIntent::Did(String::from("Synced feature/login-page with origin")),
        MenuCommand::SwitchBranch => {
            let name = row
                .and_then(|row| data::BRANCHES.get(row))
                .map_or("", |branch| branch.name);
            GalleryIntent::Did(format!("Switched to {name}"))
        }
    };
    intents.push(intent);
}

/// The shortcuts of the gallery, read once its widgets used the keys they wanted: Ctrl+K opens Quick search, Ctrl+P
/// and Ctrl+Shift+P the pickers of branches, and Ctrl+C copies the hash of the commit picked. They do nothing while an
/// overlay is open.
pub(super) fn shortcuts(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    if state.overlay.is_some() {
        return;
    }
    let control_shift = Modifiers::CONTROL | Modifiers::SHIFT;
    for key in ui.unclaimed_keys() {
        match (key.key, key.modifiers) {
            (Key::Character('k'), modifiers) if modifiers == Modifiers::CONTROL => {
                intents.push(GalleryIntent::Open(Overlay::QuickSearch));
            }
            (Key::Character('p'), modifiers) if modifiers == Modifiers::CONTROL => {
                intents.push(GalleryIntent::Open(Overlay::PickBranch));
            }
            (Key::Character('p'), modifiers) if modifiers == control_shift => {
                intents.push(GalleryIntent::Open(Overlay::PickBranches));
            }
            (Key::Character('c'), modifiers) if modifiers == Modifiers::CONTROL => {
                let row = super::picked_commit(ui);
                if row.is_some() {
                    run_command(ui, MenuCommand::CopyHash, row, intents);
                }
            }
            // The other keys are not shortcuts of the gallery.
            _ => {}
        }
    }
}

/// Escape, when nothing else used it, closes the Activity drawer, or the inspector open over a narrow window.
pub(super) fn close_on_escape(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    if state.overlay.is_some() {
        return;
    }
    let is_over = state.is_inspector_open && ui.window().width < INSPECTOR_OVER_BELOW;
    let escape = ui
        .unclaimed_keys()
        .into_iter()
        .find(|key| key.key == Key::Named(NamedKey::Escape) && key.modifiers == Modifiers::NONE);
    let Some(escape) = escape else {
        return;
    };
    if state.is_activity_open {
        ui.claim_any_key(escape);
        intents.push(GalleryIntent::CloseActivity);
    } else if is_over {
        ui.claim_any_key(escape);
        intents.push(GalleryIntent::CloseInspector);
    }
}
