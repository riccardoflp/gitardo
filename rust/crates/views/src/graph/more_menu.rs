//! The **…** menu at the end of the header (`graph.more-menu`), in the groups of its card, with what is built:
//! **Refresh** and the operations at `HEAD`; the switches of the timeline, checked while they are on, and the folds;
//! the switches of the rows; the ways to what is not elsewhere in the header; **Switch identity…**; then **Settings**
//! and **Theme…**.

use model::filter::{Kinds, ReferenceKind};
use ui::{Button, ButtonKind, MenuEntry, MenuItem, Ui};

use super::fold::{FoldAllEntry, FoldAllState, fold_all_view};
use super::intent::{GraphIntent, Switches, TimelineSwitch};
use super::switches::{FirstParentState, SelectedTrace, first_parent_view, trace_selected_view};
use crate::icons;
use crate::intent::{FilterIntent, Intent};
use crate::text::filter as filter_words;
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
    /// The switches of the timeline, while a repository is shown.
    pub timeline: Option<TimelineMenu>,
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

/// The entries of **…** about the timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimelineMenu {
    /// The switches, as they stand.
    pub switches: Switches,
    /// Whether a merge that can be folded is loaded.
    pub has_merges: bool,
    /// Whether a merge is folded.
    pub has_folds: bool,
    /// Whether every merge is folded.
    pub is_all_folded: bool,
    /// How the timeline is drawn: the folds are offered only while it has its lines.
    pub drawn: GraphDrawn,
    /// The kinds of references the timeline shows; none while the branch filter chose otherwise (`filter.kinds`).
    pub kinds: Option<Kinds>,
}

/// How the timeline is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphDrawn {
    /// With the lines of the graph.
    WithLines,
    /// As a list of what a search finds, where nothing is folded.
    AsList,
}

/// What an item of the menu does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MoreCommand {
    /// **Refresh**.
    Refresh,
    /// A switch of the timeline.
    Switch(TimelineSwitch),
    /// **Fold all merges** or **Unfold all merges**.
    FoldAll(FoldAllEntry),
    /// **Select repository…**.
    SelectRepository,
    /// **Show local branches**, **Show remote branches** or **Show tags**.
    Kind(ReferenceKind),
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

/// The words of a switch.
fn switch_words(switch: TimelineSwitch) -> &'static str {
    match switch {
        TimelineSwitch::ShowStashes => words::SHOW_STASHES,
        TimelineSwitch::FirstParent => words::FIRST_PARENT,
        TimelineSwitch::OneLane => words::ONE_LANE,
        TimelineSwitch::TraceSelected => words::TRACE_SELECTED,
        TimelineSwitch::Author => words::SHOW_AUTHOR,
        TimelineSwitch::Date => words::SHOW_DATE,
        TimelineSwitch::Hash => words::SHOW_HASH,
        TimelineSwitch::DateSeparators => words::DATE_SEPARATORS,
        TimelineSwitch::CombineUpstream => words::COMBINE_UPSTREAM,
    }
}

/// The item of a switch, with a check while it is on.
fn switch_item(switches: Switches, switch: TimelineSwitch) -> MenuEntry<'static, MoreCommand> {
    let item = MenuItem::new(switch_words(switch), MoreCommand::Switch(switch));
    MenuEntry::Item(if switches.is_on(switch) {
        item.with_icon(icons::CHOSEN)
    } else {
        item
    })
}

/// The entries of the timeline, in the order of the card: the switches of what is listed and brought out, the folds,
/// while the graph has its lines and is not on one line, then the switches of what each row shows.
fn timeline_entries(menu: TimelineMenu) -> Vec<MenuEntry<'static, MoreCommand>> {
    let mut entries = vec![MenuEntry::Separator];
    entries.extend(kind_entries(menu.kinds));
    for switch in [
        TimelineSwitch::ShowStashes,
        TimelineSwitch::FirstParent,
        TimelineSwitch::OneLane,
        TimelineSwitch::TraceSelected,
    ] {
        entries.push(switch_item(menu.switches, switch));
    }
    if menu.drawn == GraphDrawn::WithLines && !menu.switches.is_on(TimelineSwitch::OneLane) {
        let fold = MenuItem::new(words::FOLD_ALL, MoreCommand::FoldAll(FoldAllEntry::Fold));
        let fold = match (menu.is_all_folded, menu.has_merges) {
            (true, _) => fold.with_icon(icons::CHOSEN),
            (false, true) => fold,
            (false, false) => fold.disabled(),
        };
        let unfold = MenuItem::new(words::UNFOLD_ALL, MoreCommand::FoldAll(FoldAllEntry::Unfold));
        entries.push(MenuEntry::Item(fold));
        entries.push(MenuEntry::Item(if menu.has_folds { unfold } else { unfold.disabled() }));
    }
    entries.push(MenuEntry::Separator);
    for switch in [
        TimelineSwitch::Author,
        TimelineSwitch::Date,
        TimelineSwitch::Hash,
        TimelineSwitch::DateSeparators,
        TimelineSwitch::CombineUpstream,
    ] {
        entries.push(switch_item(menu.switches, switch));
    }
    entries
}

/// The kinds of references shown, each checked while it is shown: greyed out with **Chosen in the filter** while the
/// branch filter chose otherwise, and the one kind left on greyed out, since one is always on.
pub(crate) fn kind_entries(kinds: Option<Kinds>) -> Vec<MenuEntry<'static, MoreCommand>> {
    [
        (ReferenceKind::Local, filter_words::SHOW_LOCAL),
        (ReferenceKind::Remote, filter_words::SHOW_REMOTE),
        (ReferenceKind::Tag, filter_words::SHOW_TAGS),
    ]
    .into_iter()
    .map(|(kind, label)| {
        let item = MenuItem::new(label, MoreCommand::Kind(kind));
        MenuEntry::Item(match kinds {
            None => item.disabled().with_shortcut(filter_words::CHOSEN_IN_FILTER),
            Some(kinds) if kinds.has(kind) && kinds.count() == 1 => item.with_icon(icons::CHOSEN).disabled(),
            Some(kinds) if kinds.has(kind) => item.with_icon(icons::CHOSEN),
            Some(_) => item,
        })
    })
    .collect()
}

/// The **…** button and its menu, over the rectangle its caller clipped it to.
///
/// Feature cards that fill it further: `graph.more-menu` (**Branch**, **Stash**, **Undo**, the kinds of references
/// shown, the ways to the pages), `repo.schedule-reads` (what **Refresh** asks for).
///
/// Feature: graph.more-menu
pub fn more_menu_view(state: &MoreMenuState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::icon(icons::MORE, words::MORE).kind(ButtonKind::Outline);
    let entries = menu_entries(state);
    let switches = state.timeline.map_or(Switches::DEFAULT, |timeline| timeline.switches);
    match ui.menu_button(MORE_MENU, rect, &button, &entries) {
        Some(MoreCommand::Refresh) => vec![Intent::Refresh],
        Some(MoreCommand::SelectRepository) => vec![Intent::PickRepository],
        Some(MoreCommand::Kind(kind)) => vec![Intent::Filter(FilterIntent::SwitchKind(kind))],
        Some(MoreCommand::GoToCommit) => vec![Intent::Search(crate::search::SearchIntent::GoToCommit)],
        Some(MoreCommand::ActiveFileHistory) => vec![active_file_history(state.active_file)],
        Some(MoreCommand::Settings) => vec![Intent::ShowPage(crate::page::Page::Settings)],
        Some(MoreCommand::Theme) => vec![Intent::PickTheme],
        Some(MoreCommand::Operation(feature)) => vec![Intent::Feature(crate::feature::FeatureIntent::Run(
            model::contract::FeatureId(feature),
            model::contract::Target::Head,
        ))],
        Some(MoreCommand::Switch(TimelineSwitch::FirstParent)) => {
            let state = FirstParentState {
                is_on: switches.is_on(TimelineSwitch::FirstParent),
            };
            first_parent_view(&state, ui)
        }
        Some(MoreCommand::Switch(TimelineSwitch::TraceSelected)) => {
            let state = SelectedTrace {
                is_on: switches.is_on(TimelineSwitch::TraceSelected),
            };
            trace_selected_view(&state, ui)
        }
        Some(MoreCommand::Switch(switch)) => vec![Intent::Graph(GraphIntent::Switch(switch, !switches.is_on(switch)))],
        Some(MoreCommand::FoldAll(chosen)) => {
            let state = FoldAllState {
                chosen,
                is_all_folded: state.timeline.is_some_and(|timeline| timeline.is_all_folded),
            };
            fold_all_view(&state, ui)
        }
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
        let item = MenuItem::new(label, MoreCommand::Operation(feature));
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
    let item = MenuItem::new(label, MoreCommand::Operation(feature));
    MenuEntry::Item(if operations.is_running { item.disabled() } else { item })
}

/// The ways to what is not elsewhere in the header, in the order of the card: **Compare branches, tags or commits…**,
/// **Go to commit…**, **History of the active file**, **Reflog**, **Clean up branches…**, **Apply patch…**, **Manage remotes…** and, while the
/// repository has no submodule, **Add submodule…**.
fn ways_entries(operations: OperationsMenu, has_submodules: bool) -> Vec<MenuEntry<'static, MoreCommand>> {
    let mut entries = vec![
        operation_item(crate::text::compare::COMPARE_BRANCHES, "compare.choose", operations),
        MenuEntry::Item(MenuItem::new(
            crate::text::search::GO_TO_COMMIT_ENTRY,
            MoreCommand::GoToCommit,
        )),
        MenuEntry::Item(if operations.is_running {
            MenuItem::new(words::ACTIVE_FILE_HISTORY, MoreCommand::ActiveFileHistory).disabled()
        } else {
            MenuItem::new(words::ACTIVE_FILE_HISTORY, MoreCommand::ActiveFileHistory)
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

/// The entries of **…**, in the groups of the card: **Refresh**, **Branch**, **Stash** and **Undo**; the entries of the
/// timeline; the ways to the pages and **Select repository…**; **Switch identity…**; **Load pull requests from
/// GitHub** when the main remote is on GitHub; then **Settings** and **Theme…**.
fn menu_entries(state: &MoreMenuState) -> Vec<MenuEntry<'static, MoreCommand>> {
    let refresh = MenuItem::new(words::REFRESH, MoreCommand::Refresh).with_icon(icons::READ_AGAIN);
    let mut entries = vec![MenuEntry::Item(if state.can_refresh {
        refresh
    } else {
        refresh.disabled()
    })];
    if let Some(operations) = state.operations {
        entries.extend(operation_entries(operations));
    }
    if let Some(timeline) = &state.timeline {
        entries.extend(timeline_entries(*timeline));
    }
    if state.operations.is_some() || state.can_select_repository {
        entries.push(MenuEntry::Separator);
    }
    if let Some(operations) = state.operations {
        entries.extend(ways_entries(operations, state.has_submodules));
    }
    if state.can_select_repository {
        entries.push(MenuEntry::Item(MenuItem::new(
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
        MenuItem::new(crate::text::settings::SETTINGS_ENTRY, MoreCommand::Settings).with_shortcut("Ctrl+,"),
    ));
    entries.push(MenuEntry::Item(MenuItem::new(
        crate::text::settings::THEME_ENTRY,
        MoreCommand::Theme,
    )));
    entries
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
