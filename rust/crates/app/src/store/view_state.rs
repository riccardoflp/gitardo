//! What the views are drawn from: the state of the window of [`views`], made from the store for each frame. The words
//! it needs that the store does not keep (the names of the repositories, the recent workspaces by their folders, what
//! is running) are made first, in a [`ViewData`] the state of the views borrows.

use std::path::PathBuf;

use model::filter::kinds_shown;
use model::repo::name_repositories;
use model::state::RepositoryState;
use views::activity::{ActivityState, RanTask, RunningTask, StatusBarState};
use views::branch::ReferenceMenus;
use views::filter::NarrowedBar;
use views::graph::{
    Awaited, Folds, GraphState, HeadName, HistoryState, MoreMenuState, Reading, RepositoryEntry, SyncCounts,
    TimelineMenu, ToolbarState, TraceShown, ViewMenuState, WhereState,
};
use views::log::{LogLine, LogState};
use views::page::Page;
use views::panel::BottomPanelState;
use views::repo::{NoRepositoryState, OpenFolderQuestion, RepositoryPickerState, SideColumnState, SideSections};
use views::settings::{SettingRow, ThemePickerState};
use views::{SidePlace, TimelineState, WindowState};

use super::search::{self, view as search_view};
use super::state::{Dialog, Shown, State};
use super::timeline::workspace_panes;
use super::view_parts::{branch_picker, pull_way, rail, selected_details, view_bar};
use crate::persisted::{WorkspacePanes, same_folder};

mod content;

/// The words the views need that the store does not keep.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ViewData {
    /// The recent workspaces, each by its folders.
    recent: Vec<String>,
    /// The repositories of the workspace, by their names, the one shown marked.
    repositories: Vec<RepositoryEntry>,
    /// The words of the tabs.
    tabs: super::tabs::TabsData,
    /// The name of the repository shown, or of the first folder when the workspace has no repository.
    name: Option<String>,
    /// The folder of the repository shown.
    folder: String,
    /// The folder the dialog of an opened folder asks about.
    question_folder: String,
    /// What is running, for the status bar and the Activity tab.
    running: Vec<RunningTask>,
    /// What ran, the latest first, for the Activity tab.
    ran: Vec<RanTask>,
    /// The lines of the Command log, while the bottom panel shows it.
    log_lines: Vec<LogLine>,
    /// The settings, while the settings page is open.
    setting_rows: Vec<SettingRow>,
    /// The sizes of the panes of the workspace.
    panes: WorkspacePanes,
    /// The moment the dates are said from, in seconds since 1970.
    now: i64,
    /// The entries of Quick search.
    search: search::SearchView,
    /// The list of the Changes page, while it is open.
    changes: super::changes::ChangesData,
    /// The blame of the File page in front.
    blame: super::blame::BlameData,
    /// What the Tasks tab of the bottom panel lists.
    task_list: views::tasks::TaskList,
    /// What the picker of **Run Task…** lists, while it is open.
    quick_run: Vec<views::tasks::QuickRunEntry>,
}

impl ViewData {
    /// The words of a state, with what is running and the moment the dates are said from, in seconds since 1970.
    #[must_use]
    pub fn new(state: &State, running: Vec<RunningTask>, now: i64) -> ViewData {
        let recent = super::pages::recent_names(state);
        let roots = state
            .workspace
            .as_ref()
            .map_or_default(|workspace| workspace.folders.clone());
        let folders: Vec<PathBuf> = state.repositories().iter().map(|found| found.folder.clone()).collect();
        let shown = state.shown_folder();
        let repositories: Vec<RepositoryEntry> = name_repositories(&roots, &folders)
            .into_iter()
            .zip(folders)
            .map(|(name, folder)| RepositoryEntry {
                is_current: shown.is_some_and(|shown| same_folder(shown, &folder)),
                name,
                folder,
            })
            .collect();
        let name = repositories
            .iter()
            .find(|entry| entry.is_current)
            .map(|entry| entry.name.clone())
            .or_else(|| {
                name_repositories(&roots, roots.get(..1).unwrap_or_default())
                    .into_iter()
                    .next()
            });
        let question_folder = match &state.dialog {
            Some(Dialog::OpenFolder { opened }) => opened.name(),
            _ => String::new(),
        };
        ViewData {
            recent,
            tabs: super::tabs::TabsData::new(state, &repositories),
            repositories,
            name,
            folder: shown.map_or_default(|folder| folder.display().to_string()),
            question_folder,
            running,
            ran: Vec::new(),
            log_lines: if super::panel::is_log_shown(state) {
                super::log_lines::log_lines(&state.log)
            } else {
                Vec::new()
            },
            setting_rows: if state.pages.open.contains(&Page::Settings) {
                crate::settings::setting_rows(&state.settings, super::settings_page::shown_target(state))
            } else {
                Vec::new()
            },
            panes: workspace_panes(state),
            now,
            search: search::SearchView::new(state),
            changes: super::changes::ChangesData::new(state),
            blame: super::blame::BlameData::new(state, now),
            task_list: super::tasks_tab::task_list(state),
            quick_run: super::tasks_tab::quick_run_entries(state),
        }
    }

    /// The same words, with what ran, the latest first, for the Activity tab.
    #[must_use]
    pub fn with_ran(self, ran: Vec<RanTask>) -> ViewData {
        ViewData { ran, ..self }
    }

    /// The name of the repository shown, for the title of the window.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The state of the window of the views.
    #[must_use]
    pub fn window_state<'a>(&'a self, state: &'a State) -> WindowState<'a> {
        WindowState {
            view_bar: view_bar(state),
            side: self.side_column(state),
            side_place: self.side_place(state),
            tabs_picker: self.tabs.picker(state),
            header: self.header(state),
            status: StatusBarState {
                running: &self.running,
                identity: state.shown_read().map(|read| &read.state.identity),
                is_running: state.is_operation_running,
                said: None,
            },
            content: self.content(state),
            panel: self.panel(state),
            theme_picker: (state.dialog == Some(Dialog::ThemePicker)).then_some(ThemePickerState {
                current: crate::settings::theme_value(state.settings.theme),
            }),
            picker: (state.dialog == Some(Dialog::Picker)).then_some(RepositoryPickerState {
                repositories: &self.repositories,
            }),
            question: matches!(state.dialog, Some(Dialog::OpenFolder { .. })).then_some(OpenFolderQuestion {
                folder: &self.question_folder,
            }),
            notice: state.notice,
            go_to_commit: search_view::go_to_commit(state),
            chooser: super::compare::chooser::chooser_state(state),
            feature: super::feature::question_state(state)
                .map(views::FeatureDialog::Question)
                .or_else(|| {
                    super::feature::plan_shown(state)
                        .map(|plan| views::FeatureDialog::Plan(views::feature::ConfirmState { plan }))
                }),
            said: super::feature::said_state(state),
            large_diff: super::diff_page::large_question(state),
            large_merge: super::merge_page::large_question(state),
            close_file: super::file_page::close_question(state),
            co_authors: super::co_authors::picker_state(state),
            caption: state.caption,
        }
    }

    /// Where the side column is, as the user left it, for the window and for the button of the title row of a page.
    fn side_place(&self, state: &State) -> SidePlace {
        SidePlace {
            width: self.panes.sizes.rail,
            is_hidden: self.panes.is_rail_hidden,
            is_open_over: state.is_side_column_over,
        }
    }

    /// The side column of an open workspace: its repositories, and the changes and the rail of the one read
    /// (`repo.side-column`).
    fn side_column<'a>(&'a self, state: &'a State) -> Option<SideColumnState<'a>> {
        let tabs = self.tabs.island(state)?;
        let branches = match state.workspace.as_ref().map(|workspace| &workspace.shown) {
            Some(Shown::Read(read)) => Some(Box::new(rail(state, read, self.now))),
            _ => None,
        };
        Some(SideColumnState {
            tabs,
            changes: Box::new(self.changes.page_state(state)),
            branches,
            sections: SideSections {
                repositories_height: self.panes.sizes.repositories,
                changes_height: self.panes.changes_section,
                is_repositories_folded: self.panes.folds.is_repositories_folded,
                is_changes_folded: self.panes.folds.is_changes_folded,
                is_branches_folded: self.panes.folds.is_branches_folded,
            },
        })
    }

    /// The bottom panel, while it is shown.
    fn panel<'a>(&'a self, state: &'a State) -> Option<BottomPanelState<'a>> {
        let look = super::panel::look(state);
        look.is_shown.then(|| BottomPanelState {
            tab: look.tab,
            height: look.height,
            activity: ActivityState {
                running: &self.running,
                ran: &self.ran,
            },
            log: LogState {
                lines: &self.log_lines,
                dropped: state.log.dropped(),
                at: super::log_lines::log_at(state, self.log_lines.len()),
            },
            terminal: super::terminals::view_state(state),
            tasks: views::tasks::TasksTabState {
                list: &self.task_list,
                terminal: super::tasks_tab::terminal_state(state),
            },
        })
    }

    /// The header.
    fn header<'a>(&'a self, state: &'a State) -> ToolbarState<'a> {
        let read = state.shown_read().map(|read| read.state.as_ref());
        let upstream = read.and_then(checked_out_upstream);
        ToolbarState {
            place: self.tabs.island(state).map(|tabs| WhereState {
                tabs: tabs.tabs,
                head: read.map(head_name),
                folder: &self.folder,
            }),
            sync: read.map(|_| {
                let (ahead, behind) = upstream.unwrap_or_default();
                SyncCounts { behind, ahead }
            }),
            sync_menu: read.map(|_| views::graph::SyncMenu {
                is_running: state.is_operation_running,
                built: views::graph::SYNC_FEATURES.map(|feature| features::operation(feature).is_some()),
                pull_way: pull_way(&state.settings.features),
            }),
            view: ViewMenuState {
                timeline: read.map(|_| timeline_menu(state)),
            },
            more: MoreMenuState {
                can_refresh: state.workspace.is_some(),
                can_select_repository: state.workspace.is_some(),
                operations: read.map(|read| views::graph::OperationsMenu {
                    is_running: state.is_operation_running,
                    has_commits: read.head.commit.is_some(),
                    can_stash: features::operation("stash.push").is_some() && !read.working_tree.is_clean(),
                }),
                has_submodules: read.is_some_and(|read| !read.submodules.is_empty()),
                main_remote: if read.is_some() && state.github.repository.is_some() {
                    views::graph::MainRemote::OnGitHub
                } else {
                    views::graph::MainRemote::Elsewhere
                },
                active_file: state.file_in_front(),
            },
            quick: self.search.quick(state),
            quick_run: super::tasks_tab::quick_run_state(state, &self.quick_run),
        }
    }

    /// The timeline, the branch rail and the inspector of the repository shown.
    fn timeline<'a>(&'a self, state: &'a State) -> TimelineState<'a> {
        let shown = state.workspace.as_ref().map(|workspace| &workspace.shown);
        match shown {
            Some(Shown::Read(read)) => TimelineState::Repository {
                graph: GraphState::Listed(Box::new(self.history(state, read.state.head.commit.is_none()))),
                inspector: Box::new(super::inspector::details_state(
                    state,
                    selected_details(&state.timeline),
                )),
                panes: self.panes.sizes,
                filter: Box::new(branch_picker(state)),
                bar: state.timeline.shown.as_ref().map(|shown| NarrowedBar {
                    shown,
                    listed: state
                        .timeline
                        .read_rows()
                        .iter()
                        .filter(|row| row.id().is_some())
                        .count(),
                    has_more: state.timeline.has_more,
                }),
                search: Box::new((search_view::search_box(state), search_view::list_mode(state))),
                conflicts: super::conflicts::banner_state(state, read),
                bisect: read.state.bisect.as_ref().map(|bisect| views::bisect::BisectBanner {
                    bisect,
                    head: read.state.head.commit.as_ref(),
                    is_running: state.is_operation_running,
                }),
                worktree: super::worktrees::banner_state(state, read),
                beside: None,
            },
            Some(Shown::NoRepository) => TimelineState::NoRepository(NoRepositoryState::NoRepository),
            Some(Shown::Looking) if state.reads.is_not_read => TimelineState::NoRepository(NoRepositoryState::NotRead),
            Some(Shown::Looking | Shown::Reading { .. }) | None => {
                TimelineState::NoRepository(NoRepositoryState::Loading)
            }
        }
    }

    /// The commits of the timeline and everything about them.
    fn history<'a>(&'a self, state: &'a State, has_no_commits: bool) -> HistoryState<'a> {
        let timeline = &state.timeline;
        let reading = match (timeline.is_reading_again, timeline.more_job, &timeline.stream) {
            (true, _, _) => Reading::Again,
            (false, Some(_), _) => Reading::More,
            (false, None, Some(_)) => Reading::Pages,
            (false, None, None) => Reading::Nothing,
        };
        HistoryState {
            rows: &timeline.rows,
            layout: &timeline.layout,
            folds: Folds {
                folded: &timeline.folded,
                is_all_folded: timeline.is_all_folded,
            },
            has_no_commits,
            has_more: timeline.has_more,
            reading,
            asked_at: timeline.asked_at,
            selected: timeline.selected.as_ref(),
            several: &timeline.several,
            reveal: timeline.reveal,
            trace: timeline.trace.as_ref().map(|(cells, strength)| TraceShown {
                cells,
                strength: *strength,
            }),
            hover: timeline.hover.as_ref(),
            hover_left_at: timeline.hover_left_at,
            label_list: timeline.label_list.as_ref(),
            tag_marks: state.shown_folder().and_then(|folder| state.rail.tag_marks.get(folder)),
            label_list_closed_at: timeline.label_list_closed_at,
            rail_hover: None,
            switches: state.settings.switches,
            graph_width: self.panes.graph_width,
            now: self.now,
            found: &state.search.found_rows,
            dims_others: search_view::dims_others(state),
            draft: timeline.draft,
            references: state.shown_read().map(|read| ReferenceMenus {
                repository: &read.state,
                open_in: state.settings.features.branch_open_in,
                window: super::worktrees::window_of(state),
                github: super::github::shown(state),
            }),
            commit_box: super::changes::commit_box(state),
            dropped: timeline.dropped.as_ref(),
            awaited: timeline.awaited.as_ref().map(|awaited| Awaited {
                rows: &awaited.rows,
                layout: &awaited.layout,
            }),
        }
    }
}

/// The switches of the menu of the view.
fn timeline_menu(state: &State) -> TimelineMenu {
    let timeline = &state.timeline;
    TimelineMenu {
        switches: state.settings.switches,
        has_merges: !timeline.layout.foldable.is_empty(),
        has_folds: !timeline.folded.is_empty(),
        is_all_folded: timeline.is_all_folded,
        kinds: kinds_shown(timeline.shown.as_ref()),
        drawn: if state.search.list.is_none() {
            views::graph::GraphDrawn::WithLines
        } else {
            views::graph::GraphDrawn::AsList
        },
    }
}

/// How far the branch checked out is from its upstream, `(ahead, behind)`, when it has one that is not gone.
fn checked_out_upstream(repository: &RepositoryState) -> Option<(u32, u32)> {
    let name = repository.checked_out_branch()?;
    let branch = repository.branches.iter().find(|branch| branch.name == name)?;
    branch
        .upstream
        .as_ref()
        .filter(|upstream| !upstream.is_gone)
        .map(|upstream| (upstream.ahead, upstream.behind))
}

/// What `HEAD` is, as the branch of the header names it.
fn head_name(repository: &RepositoryState) -> HeadName<'_> {
    match (repository.checked_out_branch(), &repository.head.commit) {
        (Some(branch), Some(_)) => HeadName::Branch(branch),
        (Some(branch), None) => HeadName::Unborn(branch),
        (None, Some(commit)) => HeadName::Detached(commit.short()),
        (None, None) => HeadName::NoCommits,
    }
}
