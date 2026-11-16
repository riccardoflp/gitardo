//! The repositories of the workspace: the island of the tabs (`repo.tabs`), what the timeline says while there is none
//! to show (`repo.no-repository`), the picker of the repositories (`repo.select`), and the question asked when a folder
//! is opened while the window shows a workspace (`repo.open-folder`).

mod fit;
mod tabs;

pub use fit::{Fit, TAB_LEAST, fit_tabs};
pub use tabs::{ONE_BUTTON_BELOW, TAB_MOST, TabEntry, TabMemberEntry, TabsState, cut_at_start, tabs_view};

use ui::layout::{Align, Area, place};
use ui::{Button, Dialog, DialogButton, DialogRole, Label, Picker, PickerAnswer, PickerItem, PickerMode, Tone, Ui};

use crate::graph::RepositoryEntry;
use crate::intent::{FolderAnswer, Intent};
use crate::text::header;
use crate::text::repo as words;

/// What the timeline says in place of the commits while the workspace has no repository to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoRepositoryState {
    /// The repositories are being looked for, or the first one is being read.
    Loading,
    /// The workspace has no repository, or the one it had could not be read.
    NoRepository,
    /// The window opened with `readOnStartup` off: the repository is named but not read (`repo.read-state`).
    NotRead,
}

/// What the timeline shows in a workspace with no repository: **No Git repository in this workspace** with **Check
/// again**, or **Loading repository…** while the repositories are looked for and the first one is read, or **Not read
/// yet** with **Read the repository** when the window opened with `readOnStartup` off. The branch rail beside it is
/// empty.
///
/// Feature: repo.no-repository
pub fn no_repository_view(state: &NoRepositoryState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let line = theme.sizes.row_height;
    let height = match state {
        NoRepositoryState::Loading => line,
        NoRepositoryState::NoRepository | NoRepositoryState::NotRead => {
            line + spacing.large + theme.sizes.control_height
        }
    };
    let block = place(rect, (rect.width, height), (Align::Center, Align::Center));
    let mut area = Area::with_gap(block, spacing.large);
    let said = match state {
        NoRepositoryState::Loading => words::LOADING_REPOSITORY,
        NoRepositoryState::NoRepository => words::NO_REPOSITORY,
        NoRepositoryState::NotRead => words::NOT_READ_YET,
    };
    let _ = ui.label(area.top(line), &Label::new(said).tone(Tone::Muted).align(Align::Center));
    if *state == NoRepositoryState::Loading {
        return Vec::new();
    }
    let (button, intent) = if *state == NoRepositoryState::NotRead {
        (Button::primary(words::READ_THE_REPOSITORY), Intent::ReadRepository)
    } else {
        (Button::secondary(words::CHECK_AGAIN), Intent::CheckAgain)
    };
    let (width, button_height) = ui.button_size(&button);
    let cell = place(area.rest(), (width, button_height), (Align::Center, Align::Start));
    if ui.button("check again", cell, &button).clicked {
        vec![intent]
    } else {
        Vec::new()
    }
}

/// What the picker of the repositories lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepositoryPickerState<'a> {
    /// The repositories of the workspace, in their order.
    pub repositories: &'a [RepositoryEntry],
}

/// The picker titled **Repository shown in Gitardo**: each repository by its name, with its folder beside it, and the
/// one shown marked **current**. Choosing one shows it.
///
/// Feature card that fills it further: `repo.select` (a worktree shown from outside the workspace, listed last as
/// **worktree, current**; matching what is typed against the folders too).
pub fn repository_picker_view(state: &RepositoryPickerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let descriptions: Vec<String> = state
        .repositories
        .iter()
        .map(|repository| {
            let folder = repository.folder.to_string_lossy();
            if repository.is_current {
                format!("{} \u{b7} {folder}", header::CURRENT)
            } else {
                folder.into_owned()
            }
        })
        .collect();
    let items: Vec<PickerItem<'_>> = state
        .repositories
        .iter()
        .zip(&descriptions)
        .map(|(repository, description)| PickerItem {
            label: &repository.name,
            description,
        })
        .collect();
    let picker = Picker {
        placeholder: words::PICKER_TITLE,
        items: &items,
        mode: PickerMode::Single,
        query: "",
        checked: &[],
    };
    match ui.picker("repository picker", &picker) {
        Some(PickerAnswer::Chose(index)) => state
            .repositories
            .get(index)
            .map_or_default(|repository| vec![Intent::SelectRepository(repository.folder.clone())]),
        Some(PickerAnswer::Cancelled | PickerAnswer::ChoseMany(_)) => vec![Intent::ClosePicker],
        None => Vec::new(),
    }
}

/// Which picker of the tabs is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabsPickerKind {
    /// **Repositories to open in tabs**, all checked (`repo.choose-tabs`).
    Choose,
    /// **Open a repository in a tab**, one of those with no tab (`repo.open-tab`).
    Open,
}

/// What a picker of the tabs lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabsPickerState<'a> {
    /// Which picker.
    pub kind: TabsPickerKind,
    /// The repositories it lists, in the order they were found.
    pub repositories: &'a [RepositoryEntry],
}

/// The picker of the repositories to open in tabs, each by its name with its folder beside it, all checked at first,
/// `Enter` opening the ones checked (`repo.choose-tabs`); or the picker of a repository to open in a tab
/// (`repo.open-tab`).
pub fn tabs_picker_view(state: &TabsPickerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let descriptions: Vec<String> = state
        .repositories
        .iter()
        .map(|repository| repository.folder.to_string_lossy().into_owned())
        .collect();
    let items: Vec<PickerItem<'_>> = state
        .repositories
        .iter()
        .zip(&descriptions)
        .map(|(repository, description)| PickerItem {
            label: &repository.name,
            description,
        })
        .collect();
    let all: Vec<usize> = (0..items.len()).collect();
    let (placeholder, mode, checked) = match state.kind {
        TabsPickerKind::Choose => (words::CHOOSE_TABS_TITLE, PickerMode::Multiple, all.as_slice()),
        TabsPickerKind::Open => (words::OPEN_TAB_TITLE, PickerMode::Single, &[][..]),
    };
    let picker = Picker {
        placeholder,
        items: &items,
        mode,
        query: "",
        checked,
    };
    match (
        ui.picker(("tabs picker", state.kind == TabsPickerKind::Choose), &picker),
        state.kind,
    ) {
        (Some(PickerAnswer::ChoseMany(chosen)), _) => vec![Intent::TabsChosen(Some(chosen))],
        (Some(PickerAnswer::Cancelled), TabsPickerKind::Choose) => vec![Intent::TabsChosen(None)],
        (Some(PickerAnswer::Chose(index)), _) => state
            .repositories
            .get(index)
            .map_or_default(|repository| vec![Intent::OpenTab(repository.folder.clone())]),
        (Some(PickerAnswer::Cancelled), TabsPickerKind::Open) => vec![Intent::ClosePicker],
        (None, _) => Vec::new(),
    }
}

/// The question asked when a folder is opened while the window shows a workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenFolderQuestion<'a> {
    /// The name of the folder chosen.
    pub folder: &'a str,
}

/// The question **Replace the workspace** or **Add to the workspace**, asked when a folder is opened while the window
/// shows a workspace; replacing is the default, which Enter chooses.
pub fn open_folder_question_view(state: &OpenFolderQuestion<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let title = words::open_question(state.folder);
    let buttons = [
        DialogButton {
            label: words::CANCEL,
            role: DialogRole::Cancel,
        },
        DialogButton {
            label: words::ADD,
            role: DialogRole::Secondary,
        },
        DialogButton {
            label: words::REPLACE,
            role: DialogRole::Primary,
        },
    ];
    let dialog = Dialog {
        title: &title,
        body_height: 0.0,
        buttons: &buttons,
    };
    let response = ui.dialog("open folder question", &dialog, |_, _| {});
    let answer = match response.pressed {
        _ if response.is_cancelled => Some(FolderAnswer::Cancel),
        Some(1) => Some(FolderAnswer::Add),
        Some(2) => Some(FolderAnswer::Replace),
        Some(_) | None => None,
    };
    answer.map_or_default(|answer| vec![Intent::AnswerOpenFolder(answer)])
}
