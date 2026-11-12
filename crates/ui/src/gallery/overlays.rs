//! What opens over the gallery: the menu of the toolbar with its submenu of themes, the context menu of the rows, a
//! dialog, a confirmation of something that cannot be undone, and pickers of one branch or of several.

use render::{Icon, ThemeName};

use super::{Field, GalleryIntent, GalleryState, Overlay};
use crate::frame::Ui;
use crate::input::{Key, Modifiers};
use crate::layout::Area;
use crate::widget::Tone;
use crate::widget::dialog::{ConfirmAnswer, ConfirmStyle, Confirmation, Dialog, DialogButton, DialogRole};
use crate::widget::input_box::InputBox;
use crate::widget::label::Label;
use crate::widget::list::Selection;
use crate::widget::menu::{MenuEntry, MenuItem};
use crate::widget::metrics;
use crate::widget::picker::{Picker, PickerAnswer, PickerItem, PickerMode};

/// What the items of the menus of the gallery do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuCommand {
    /// Opens an overlay.
    Open(Overlay),
    /// Opens the find bar.
    Find,
    /// Changes the theme.
    Theme(ThemeName),
    /// Copies the hash of the row the menu was opened on.
    CopyHash,
    /// Cherry-picks the commit of the row.
    CherryPick,
    /// Resets the branch to the commit of the row, in a way.
    Reset(&'static str),
    /// Reverts the commit of the row.
    Revert,
    /// Signs in to GitHub.
    SignIn,
}

/// The branches the pickers offer, with what they say about each.
pub(super) const BRANCHES: [PickerItem<'static>; 10] = [
    PickerItem {
        label: "main",
        description: "origin/main, 2 hours ago",
    },
    PickerItem {
        label: "feature/login-page",
        description: "3 ahead",
    },
    PickerItem {
        label: "feature/search",
        description: "1 ahead, 4 behind",
    },
    PickerItem {
        label: "fix/parser-empty-section",
        description: "yesterday",
    },
    PickerItem {
        label: "release/2.4",
        description: "tag v2.4.0",
    },
    PickerItem {
        label: "experiment/graph-colors",
        description: "last week",
    },
    PickerItem {
        label: "feature/file-history",
        description: "worktree",
    },
    PickerItem {
        label: "docs/readme",
        description: "merged",
    },
    PickerItem {
        label: "refactor/engine-plans",
        description: "2 weeks ago",
    },
    PickerItem {
        label: "hotfix/fetch-timeout",
        description: "origin/hotfix/fetch-timeout",
    },
];

/// The key of the context menu of the rows.
pub(super) const ROW_MENU: &str = "row menu";
/// The key of the find bar.
pub(super) const FIND: &str = "find";
/// The key of the pickers.
pub(super) const PICKER: &str = "picker";

/// The entries of the menu of the toolbar, with its submenu of themes.
pub(super) fn more_menu<'menu>(
    themes: &'menu [MenuEntry<'menu, MenuCommand>; 5],
) -> [MenuEntry<'menu, MenuCommand>; 9] {
    [
        MenuEntry::Item(
            MenuItem::new("Create a branch", MenuCommand::Open(Overlay::CreateBranch)).with_icon(Icon::Branch),
        ),
        MenuEntry::Item(MenuItem::new(
            "Delete the branch",
            MenuCommand::Open(Overlay::ConfirmDelete),
        )),
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
        MenuEntry::Separator,
        MenuEntry::Item(MenuItem::new("Sign in to GitHub", MenuCommand::SignIn).disabled()),
    ]
}

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

/// The entries of the submenu of the ways to reset.
pub(super) fn reset_menu() -> [MenuEntry<'static, MenuCommand>; 3] {
    [
        MenuEntry::Item(MenuItem::new("Soft", MenuCommand::Reset("soft"))),
        MenuEntry::Item(MenuItem::new("Mixed", MenuCommand::Reset("mixed"))),
        MenuEntry::Item(MenuItem::new("Hard", MenuCommand::Reset("hard"))),
    ]
}

/// The place of the submenu of the ways to reset in the context menu of the rows.
pub(super) const RESET_ENTRY: usize = 4;

/// The entries of the context menu of the rows.
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

/// Does what a command chosen in a menu asks; the row is the one the context menu was opened on.
pub(super) fn run_command(ui: &mut Ui<'_>, command: MenuCommand, row: Option<usize>, intents: &mut Vec<GalleryIntent>) {
    let hash = row.map(super::rows::short_hash).unwrap_or_default();
    let intent = match command {
        MenuCommand::Open(overlay) => GalleryIntent::Open(overlay),
        MenuCommand::Find => {
            ui.open_find(FIND);
            return;
        }
        MenuCommand::SignIn => return,
        MenuCommand::Theme(theme) => GalleryIntent::ChoseTheme(theme),
        MenuCommand::CopyHash => {
            ui.copy_to_clipboard(hash.clone());
            GalleryIntent::Did(format!("Copied {hash}"))
        }
        MenuCommand::CherryPick => GalleryIntent::Did(format!("Cherry-picked {hash}")),
        MenuCommand::Reset(mode) => GalleryIntent::Did(format!("Reset main to {hash}, {mode}")),
        MenuCommand::Revert => GalleryIntent::Did(format!("Reverted {hash}")),
    };
    intents.push(intent);
}

/// The shortcuts of the gallery, read once its widgets used the keys they wanted: Ctrl+P and Ctrl+Shift+P open the
/// pickers, and Ctrl+C copies the hash of the row with the cursor of the list. They do nothing while an overlay is
/// open.
pub(super) fn shortcuts(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    if state.overlay.is_some() {
        return;
    }
    let control_shift = Modifiers::CONTROL | Modifiers::SHIFT;
    for key in ui.unclaimed_keys() {
        match (key.key, key.modifiers) {
            (Key::Character('p'), modifiers) if modifiers == Modifiers::CONTROL => {
                intents.push(GalleryIntent::Open(Overlay::PickBranch));
            }
            (Key::Character('p'), modifiers) if modifiers == control_shift => {
                intents.push(GalleryIntent::Open(Overlay::PickBranches));
            }
            (Key::Character('c'), modifiers) if modifiers == Modifiers::CONTROL => {
                let row = ui
                    .memory_view()
                    .list_selection(super::widget_id("commits"))
                    .and_then(Selection::cursor);
                if row.is_some() {
                    run_command(ui, MenuCommand::CopyHash, row, intents);
                }
            }
            _ => {}
        }
    }
}

/// Declares the overlay open, if any.
pub(super) fn overlay(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    match state.overlay {
        None => {}
        Some(Overlay::CreateBranch) => create_branch(state, ui, intents),
        Some(Overlay::ConfirmDelete) => confirm_delete(ui, intents),
        Some(Overlay::PickBranch) => pick(ui, PickerMode::Single, intents),
        Some(Overlay::PickBranches) => pick(ui, PickerMode::Multiple, intents),
    }
}

/// A dialog that asks for the name of a new branch.
fn create_branch(state: &GalleryState, ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let theme = *ui.theme();
    let control = metrics::control_height(&theme);
    let row = theme.sizes.row_height;
    let buttons = [
        DialogButton {
            label: "Cancel",
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: "Create branch",
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: "Create a branch",
        body_height: row + control + theme.sizes.spacing.medium,
        buttons: &buttons,
    };
    let response = ui.dialog("create branch", &dialog, |ui, body| {
        let mut area = Area::with_gap(body.rect, theme.sizes.spacing.medium);
        let _ = ui.label(area.top(row), &Label::new("Name of the new branch").tone(Tone::Muted));
        let input = InputBox {
            text: &state.new_branch,
            placeholder: "Branch name",
            message: None,
        };
        let field = area.top(control);
        if body.is_opening {
            let id = ui.id("name");
            ui.request_focus(id);
        }
        if let Some(text) = ui.input_box("name", field, &input).changed_text {
            intents.push(GalleryIntent::Typed(Field::NewBranch, text));
        }
    });
    if response.is_cancelled {
        intents.push(GalleryIntent::Close);
    } else if response.pressed.is_some() {
        intents.push(GalleryIntent::Did(format!("Created {}", state.new_branch.trim())));
    }
}

/// A confirmation of the deletion of a branch, with the commands that will run.
fn confirm_delete(ui: &mut Ui<'_>, intents: &mut Vec<GalleryIntent>) {
    let commands = [
        "git branch --delete --force feature/login-page",
        "git push origin --delete feature/login-page",
    ];
    let confirmation = Confirmation {
        title: "Delete feature/login-page?",
        message: "The branch and its remote branch on origin are deleted. Its commits are not merged anywhere: Undo \
                  can bring the branch back, the remote branch only by pushing it again.",
        commands: &commands,
        confirm: "Delete",
        style: ConfirmStyle::Danger,
    };
    match ui.confirm("confirm delete", &confirmation) {
        Some(ConfirmAnswer::Confirmed) => intents.push(GalleryIntent::Did(String::from("Deleted feature/login-page"))),
        Some(ConfirmAnswer::Cancelled) => intents.push(GalleryIntent::Close),
        None => {}
    }
}

/// A picker of one branch or of several.
fn pick(ui: &mut Ui<'_>, mode: PickerMode, intents: &mut Vec<GalleryIntent>) {
    let placeholder = match mode {
        PickerMode::Single => "Switch to a branch",
        PickerMode::Multiple => "Choose the branches to show in the graph",
    };
    let picker = Picker {
        placeholder,
        items: &BRANCHES,
        mode,
    };
    let label = |index: usize| BRANCHES.get(index).map_or("", |item| item.label);
    match ui.picker(PICKER, &picker) {
        Some(PickerAnswer::Chose(index)) => intents.push(GalleryIntent::Did(format!("Switched to {}", label(index)))),
        Some(PickerAnswer::ChoseMany(chosen)) => {
            let names: Vec<&str> = chosen.into_iter().map(label).collect();
            intents.push(GalleryIntent::Did(format!("Showing {}", names.join(", "))));
        }
        Some(PickerAnswer::Cancelled) => intents.push(GalleryIntent::Close),
        None => {}
    }
}
