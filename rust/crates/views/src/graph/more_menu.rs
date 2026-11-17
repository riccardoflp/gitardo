//! The **…** menu at the end of the header (`graph.more-menu`): the actions alone, in the groups of its card:
//! **Refresh** and the operations at `HEAD`; the ways to what is not elsewhere in the header; **Switch identity…**;
//! **Load pull requests from GitHub**; then **Settings** and **Theme…**. The switches of the timeline are in the menu
//! of the view beside it (`graph.view-menu`, D66).

use ui::{Button, ButtonKind, MenuEntry, Ui};

use crate::icons;
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
    /// **Branch**, **Stash** and **Undo**, while a repository is shown.
    pub operations: Option<OperationsMenu>,
    /// Whether the repository has submodules: **Add submodule…** is then in the Submodules list, not here.
    pub has_submodules: bool,
    /// Where the main remote is: **Load pull requests from GitHub** is offered only on GitHub
    /// (`github.load-pull-requests`).
    pub main_remote: MainRemote,
    /// The File page in front, whose history **History of the active file** shows; with none, the operation says to
    /// open a file (`file-history.file`).
    pub active_file: Option<crate::file_view::FilePageId>,
}

/// Where the main remote of the repository shown is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainRemote {
    /// On another host, or there is none, or it is not known yet.
    #[default]
    Elsewhere,
    /// On github.com.
    OnGitHub,
}

/// What **Branch**, **Stash** and **Undo** may do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OperationsMenu {
    /// Whether an operation runs, which disables the entries that change something.
    pub is_running: bool,
    /// Whether the repository has a commit: **Branch** and **Undo** are disabled before the first.
    pub has_commits: bool,
    /// Whether there is something to stash, and **Stash** is built.
    pub can_stash: bool,
}

/// What an item of the menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoreCommand {
    /// **Refresh**.
    Refresh,
    /// **Select repository…**.
    SelectRepository,
    /// **Go to commit…** (`search.go-to-commit`).
    GoToCommit,
    /// **History of the active file** (`file-history.file`).
    ActiveFileHistory,
    /// **Settings** (`settings.edit`).
    Settings,
    /// **Theme…** (`settings.theme`).
    Theme,
    /// **Branch**, **Stash** or **Undo**: the operation of the catalog it runs at `HEAD`.
    Operation(&'static str),
}

/// The **…** button and its menu, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it further: `graph.more-menu` (**Branch**, **Stash**, **Undo**, the ways to the pages),
/// `repo.schedule-reads` (what **Refresh** asks for).
///
/// Feature: graph.more-menu
pub fn more_menu_view(state: &MoreMenuState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::icon(icons::MORE, words::MORE).kind(ButtonKind::Outline);
    let entries = menu_entries(state);
    match ui.menu_button(MORE_MENU, rect, &button, &entries) {
        Some(MoreCommand::Refresh) => vec![Intent::Refresh],
        Some(MoreCommand::SelectRepository) => vec![Intent::PickRepository],
        Some(MoreCommand::GoToCommit) => vec![Intent::Search(crate::search::SearchIntent::GoToCommit)],
        Some(MoreCommand::ActiveFileHistory) => vec![active_file_history(state.active_file)],
        Some(MoreCommand::Settings) => vec![Intent::ShowPage(crate::page::Page::Settings)],
        Some(MoreCommand::Theme) => vec![Intent::PickTheme],
        Some(MoreCommand::Operation(feature)) => vec![Intent::Feature(crate::feature::FeatureIntent::Run(
            model::contract::FeatureId(feature),
            model::contract::Target::Head,
        ))],
        None => Vec::new(),
    }
}

/// **History of the active file**: the history of the file of the File page in front, or, with none, the operation
/// on nothing, which says to open a file.
fn active_file_history(active_file: Option<crate::file_view::FilePageId>) -> Intent {
    let feature = model::contract::FeatureId("file-history.file");
    match active_file {
        Some(page) => Intent::Blame(crate::blame::BlameIntent::Run(page, feature, None)),
        None => Intent::Feature(crate::feature::FeatureIntent::Run(
            feature,
            model::contract::Target::Head,
        )),
    }
}

/// **Branch**, **Stash** and **Undo**, each disabled while an operation runs and when it has nothing to work on.
fn operation_entries(menu: OperationsMenu) -> Vec<MenuEntry<'static, MoreCommand>> {
    [
        (words::BRANCH, "branch.create", menu.has_commits),
        (words::STASH, "stash.push", menu.can_stash),
        (words::UNDO, "undo.last", menu.has_commits),
    ]
    .into_iter()
    .map(|(label, feature, can_run)| {
        let item = icons::operation_item(label, feature, MoreCommand::Operation(feature));
        MenuEntry::Item(if can_run && !menu.is_running {
            item
        } else {
            item.disabled()
        })
    })
    .collect()
}

/// An entry that runs an operation at `HEAD`, which cannot be clicked while an operation runs.
fn operation_item(
    label: &'static str,
    feature: &'static str,
    operations: OperationsMenu,
) -> MenuEntry<'static, MoreCommand> {
    let item = icons::operation_item(label, feature, MoreCommand::Operation(feature));
    MenuEntry::Item(if operations.is_running { item.disabled() } else { item })
}

/// The ways to what is not elsewhere in the header, in the order of the card: **Compare branches, tags or commits…**,
/// **Go to commit…**, **History of the active file**, **Reflog**, **Clean up branches…**, **Apply patch…**, **Manage
/// remotes…** and, while the repository has no submodule, **Add submodule…**.
fn ways_entries(operations: OperationsMenu, has_submodules: bool) -> Vec<MenuEntry<'static, MoreCommand>> {
    let mut entries = vec![
        operation_item(crate::text::compare::COMPARE_BRANCHES, "compare.choose", operations),
        MenuEntry::Item(icons::action(
            icons::COMMIT,
            crate::text::search::GO_TO_COMMIT_ENTRY,
            MoreCommand::GoToCommit,
        )),
        MenuEntry::Item({
            let history = icons::operation_item(
                words::ACTIVE_FILE_HISTORY,
                "file-history.file",
                MoreCommand::ActiveFileHistory,
            );
            if operations.is_running {
                history.disabled()
            } else {
                history
            }
        }),
        operation_item(crate::text::bisect::REFLOG, "reflog.browse", operations),
        operation_item(crate::text::branch::CLEAN_UP_BRANCHES, "branch.clean-up", operations),
        operation_item(crate::text::patch::APPLY_PATCH, "patch.apply", operations),
        operation_item(words::MANAGE_REMOTES, "remote.manage", operations),
    ];
    if !has_submodules {
        entries.push(operation_item(
            crate::text::rail::ADD_SUBMODULE,
            "submodule.add",
            operations,
        ));
    }
    entries
}

/// The entries of **…**, in the groups of the card: **Refresh**, **Branch**, **Stash** and **Undo**; the ways to the
/// pages and **Select repository…**; **Switch identity…**; **Load pull requests from GitHub** when the main remote is
/// on GitHub; then **Settings** and **Theme…**.
fn menu_entries(state: &MoreMenuState) -> Vec<MenuEntry<'static, MoreCommand>> {
    let refresh = icons::action(icons::READ_AGAIN, words::REFRESH, MoreCommand::Refresh);
    let mut entries = vec![MenuEntry::Item(if state.can_refresh {
        refresh
    } else {
        refresh.disabled()
    })];
    if let Some(operations) = state.operations {
        entries.extend(operation_entries(operations));
    }
    if state.operations.is_some() || state.can_select_repository {
        entries.push(MenuEntry::Separator);
    }
    if let Some(operations) = state.operations {
        entries.extend(ways_entries(operations, state.has_submodules));
    }
    if state.can_select_repository {
        entries.push(MenuEntry::Item(icons::action(
            icons::WORKTREE,
            words::SELECT_REPOSITORY,
            MoreCommand::SelectRepository,
        )));
    }
    if let Some(operations) = state.operations {
        entries.push(MenuEntry::Separator);
        entries.push(operation_item(
            crate::text::changes::SWITCH_IDENTITY,
            "identity.switch",
            operations,
        ));
        if state.main_remote == MainRemote::OnGitHub {
            entries.push(MenuEntry::Separator);
            entries.push(operation_item(
                crate::text::github::LOAD_PULL_REQUESTS,
                "github.load-pull-requests",
                operations,
            ));
        }
    }
    entries.push(MenuEntry::Separator);
    entries.push(MenuEntry::Item(
        icons::action(
            icons::SETTINGS,
            crate::text::settings::SETTINGS_ENTRY,
            MoreCommand::Settings,
        )
        .with_shortcut("Ctrl+,"),
    ));
    entries.push(MenuEntry::Item(icons::action(
        icons::APPEARANCE,
        crate::text::settings::THEME_ENTRY,
        MoreCommand::Theme,
    )));
    entries
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
