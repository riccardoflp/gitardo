//! What the views are drawn from: the state of the window of [`views`], made from the store for each frame. The words
//! it needs that the store does not keep (the names of the repositories, the recent workspaces by their folders, what
//! is running) are made first, in a [`ViewData`] the state of the views borrows.

use std::path::PathBuf;

use model::filter::kinds_shown;
use model::graph::Row;
use model::repo::name_repositories;
use model::state::RepositoryState;
use views::activity::{ActivityState, RunningTask, StatusBarState};
use views::branch::ReferenceMenus;
use views::details::SelectedDetails;
use views::filter::{BranchPicker, NarrowedBar, PickerOpen};
use views::graph::{
    BranchChip, Folds, GraphState, HeadName, HistoryState, MoreMenuState, Reading, ReferencesShown, RepositoryEntry,
    RepositoryPill, Selected, SyncCounts, TimelineMenu, ToolbarState, TraceShown,
};
use views::log::{LogLine, LogState};
use views::page::{Page, PageHostState, PageState};
use views::refs::ReferencesState;
use views::repo::{NoRepositoryState, OpenFolderQuestion, RepositoryPickerState};
use views::settings::{SettingRow, SettingsPageState, ThemePickerState};
use views::welcome::WelcomeState;
use views::{Content, TimelineState, WindowState};

use super::rail::{favorites, sort_order};
use super::search::{self, view as search_view};
use super::state::{Dialog, RepositoryRead, Shown, State};
use super::timeline::{Timeline, workspace_panes};
use crate::persisted::{WorkspacePanes, same_folder};

/// The words the views need that the store does not keep.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ViewData {
    /// The recent workspaces, each by its folders.
    recent: Vec<String>,
    /// The repositories of the workspace, by their names, the one shown marked.
    repositories: Vec<RepositoryEntry>,
    /// The name of the repository shown, or of the first folder when the workspace has no repository.
    name: Option<String>,
    /// The folder of the repository shown.
    folder: String,
    /// The folder the dialog of an opened folder asks about.
    question_folder: String,
    /// What is running, for the header and the Activity drawer.
    running: Vec<RunningTask>,
    /// What ran a moment ago, the latest first, for the Activity drawer.
    finished: Vec<RunningTask>,
    /// The lines of the Command log, while the drawer shows it.
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
}

impl ViewData {
    /// The words of a state, with what is running and the moment the dates are said from, in seconds since 1970.
    #[must_use]
    pub fn new(state: &State, running: Vec<RunningTask>, now: i64) -> ViewData {
        let recent = state
            .persisted
            .recent
            .iter()
            .map(|workspace| folder_list(&workspace.folders))
            .collect();
        let roots = state
            .workspace
            .as_ref()
            .map(|workspace| workspace.folders.clone())
            .unwrap_or_default();
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
            Some(Dialog::OpenFolder { folder }) => display(folder),
            _ => String::new(),
        };
        ViewData {
            recent,
            repositories,
            name,
            folder: shown.map(|folder| display(folder)).unwrap_or_default(),
            question_folder,
            running,
            finished: Vec::new(),
            log_lines: if state.is_activity_open && state.is_log_shown {
                super::log_lines::log_lines(&state.log)
            } else {
                Vec::new()
            },
            setting_rows: if state.pages.open.contains(&Page::Settings) {
                crate::settings::setting_rows(&state.settings)
            } else {
                Vec::new()
            },
            panes: workspace_panes(state),
            now,
            search: search::SearchView::new(state),
            changes: super::changes::ChangesData::new(state),
        }
    }

    /// The same words, with what ran a moment ago, the latest first, for the Activity drawer.
    #[must_use]
    pub fn with_finished(self, finished: Vec<RunningTask>) -> ViewData {
        ViewData { finished, ..self }
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
            header: self.header(state),
            content: self.content(state),
            drawer: state.is_activity_open.then_some(ActivityState {
                running: &self.running,
                finished: &self.finished,
                log: state.is_log_shown.then_some(LogState {
                    lines: &self.log_lines,
                    dropped: state.log.dropped(),
                }),
            }),
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
            feature: super::feature::question_state(state)
                .map(views::FeatureDialog::Question)
                .or_else(|| {
                    super::feature::plan_shown(state)
                        .map(|plan| views::FeatureDialog::Plan(views::feature::ConfirmState { plan }))
                }),
            said: super::feature::said_state(state),
        }
    }

    /// The header.
    fn header<'a>(&'a self, state: &'a State) -> ToolbarState<'a> {
        let read = state.shown_read().map(|read| read.state.as_ref());
        let upstream = read.and_then(checked_out_upstream);
        ToolbarState {
            repository: RepositoryPill {
                name: self.name.as_deref(),
                repositories: if self.repositories.len() > 1 {
                    &self.repositories
                } else {
                    &[]
                },
                recent: &self.recent,
            },
            branch: read.map(|repository| BranchChip {
                head: head_name(repository),
                upstream,
                folder: &self.folder,
            }),
            status: StatusBarState { running: &self.running },
            sync: read.map(|_| {
                let (ahead, behind) = upstream.unwrap_or_default();
                SyncCounts { behind, ahead }
            }),
            sync_menu: read.map(|_| views::graph::SyncMenu {
                is_running: state.is_operation_running,
                built: ["sync.sync", "sync.fetch", "sync.pull", "sync.push"]
                    .map(|feature| features::operation(feature).is_some()),
            }),
            more: MoreMenuState {
                can_refresh: state.workspace.is_some(),
                can_select_repository: state.workspace.is_some(),
                timeline: read.map(|_| timeline_menu(state)),
                operations: read.map(|read| views::graph::OperationsMenu {
                    is_running: state.is_operation_running,
                    has_commits: read.head.commit.is_some(),
                    can_stash: features::operation("stash.push").is_some() && !read.working_tree.is_clean(),
                }),
            },
            is_activity_open: state.is_activity_open,
            references: read.map(|_| ReferencesShown {
                is_shown: !self.panes.is_rail_hidden,
            }),
            filter: read.map(|_| branch_picker(state)),
            quick: self.search.quick(state),
        }
    }

    /// What is under the header: the Welcome page while there is no workspace, the page shown, or the timeline.
    fn content<'a>(&'a self, state: &'a State) -> Content<'a> {
        let page = if state.workspace.is_none() {
            Some(Page::Welcome)
        } else {
            state.page_shown()
        };
        match page {
            Some(Page::Welcome) => Content::Page(PageHostState {
                has_trail: state.workspace.is_some(),
                page: PageState::Welcome(WelcomeState {
                    recent: &self.recent,
                    done: state.persisted.welcome_steps,
                    has_timeline: state.workspace.is_some(),
                }),
            }),
            Some(Page::Commit(id)) => match super::commit_page::page_state(state, id) {
                Some(page) => Content::Page(PageHostState {
                    has_trail: true,
                    page: PageState::Commit(page),
                }),
                None => Content::Timeline(self.timeline(state)),
            },
            Some(Page::Settings) => Content::Page(PageHostState {
                has_trail: state.workspace.is_some(),
                page: PageState::Settings(SettingsPageState {
                    rows: &self.setting_rows,
                    target: state.settings_target,
                    has_repository: state.shown_folder().is_some(),
                }),
            }),
            Some(Page::Changes) => Content::Page(PageHostState {
                has_trail: true,
                page: PageState::Changes(self.changes.page_state(state)),
            }),
            None => Content::Timeline(self.timeline(state)),
        }
    }

    /// The timeline, the branch rail and the inspector of the repository shown.
    fn timeline<'a>(&'a self, state: &'a State) -> TimelineState<'a> {
        let shown = state.workspace.as_ref().map(|workspace| &workspace.shown);
        match shown {
            Some(Shown::Read(read)) => TimelineState::Repository {
                rail: Box::new(rail(state, read)),
                graph: GraphState::Listed(Box::new(self.history(state, read.state.head.commit.is_none()))),
                inspector: super::inspector::details_state(state, selected_details(&state.timeline)),
                panes: self.panes.sizes,
                is_rail_hidden: self.panes.is_rail_hidden,
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
        let reading = match (timeline.is_reading_again, timeline.more_job) {
            (true, _) => Reading::Again,
            (false, Some(_)) => Reading::More,
            (false, None) => Reading::Nothing,
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
            reveal: timeline.reveal,
            trace: timeline.trace.as_ref().map(|(cells, strength)| TraceShown {
                cells,
                strength: *strength,
            }),
            hover: timeline.hover.as_ref(),
            hover_left_at: timeline.hover_left_at,
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
            }),
            commit_box: super::changes::commit_box(state),
        }
    }
}

/// The entries of **…** about the timeline.
fn timeline_menu(state: &State) -> TimelineMenu {
    let timeline = &state.timeline;
    TimelineMenu {
        switches: state.settings.switches,
        has_merges: !timeline.layout.foldable.is_empty(),
        has_folds: !timeline.folded.is_empty(),
        is_all_folded: timeline.is_all_folded,
        kinds: kinds_shown(timeline.shown.as_ref()),
    }
}

/// The branch rail of the repository shown.
fn rail<'a>(state: &'a State, read: &'a RepositoryRead) -> ReferencesState<'a> {
    let selected = match &state.timeline.selected {
        Some(Selected::Commit(commit)) => Some(commit),
        _ => None,
    };
    ReferencesState {
        repository: &read.state,
        references: &state.rail.references,
        lists: &state.rail.lists,
        branch_colors: &state.timeline.layout.branch_colors,
        remote_colors: &state.timeline.layout.remote_colors,
        favorites: favorites(state),
        recent: &state.rail.recent,
        submodules: &[],
        typed: &state.rail.typed,
        shown: state.timeline.shown.as_ref(),
        follows_timeline: state.settings.follows_graph_filter,
        selected,
        sort: sort_order(state),
        open_in: state.settings.features.branch_open_in,
    }
}

/// The branch filter of the header.
fn branch_picker(state: &State) -> BranchPicker<'_> {
    BranchPicker {
        shown: state.timeline.shown.as_ref(),
        references: &state.rail.references,
        open: state.rail.picker.as_ref().map(|picker| PickerOpen {
            typed: &picker.typed,
            window: picker.window,
        }),
        follows_timeline: state.settings.follows_graph_filter,
    }
}

/// What the inspector shows of the row selected, as the timeline read it.
fn selected_details(timeline: &Timeline) -> Option<SelectedDetails<'_>> {
    let commit = match timeline.selected.as_ref()? {
        Selected::Draft => return timeline.draft.map(SelectedDetails::Draft),
        Selected::Commit(commit) => commit,
    };
    let row = timeline
        .layout
        .shown_of
        .get(commit)
        .and_then(|shown| timeline.layout.read_of.get(*shown))
        .and_then(|read| timeline.rows.get(*read))
        .filter(|row| row.id() == Some(commit))
        .or_else(|| timeline.rows.iter().find(|row| row.id() == Some(commit)))?;
    match row {
        Row::Commit(commit) => Some(SelectedDetails::Commit(commit)),
        Row::Stash { commit, index } => Some(SelectedDetails::Stash { commit, index: *index }),
        Row::Uncommitted { .. } => None,
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
        (Some(branch), _) => HeadName::Branch(branch),
        (None, Some(commit)) => HeadName::Detached(commit.short()),
        (None, None) => HeadName::NoCommits,
    }
}

/// A folder as the window writes it.
fn display(folder: &std::path::Path) -> String {
    folder.display().to_string()
}

/// The folders of a workspace, on one line.
fn folder_list(folders: &[PathBuf]) -> String {
    folders
        .iter()
        .map(|folder| display(folder))
        .collect::<Vec<_>>()
        .join(", ")
}
