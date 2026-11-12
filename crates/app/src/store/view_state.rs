//! What the views are drawn from: the state of the window of [`views`], made from the store for each frame. The words
//! it needs that the store does not keep (the names of the repositories, the recent workspaces by their folders, what
//! is running) are made first, in a [`ViewData`] the state of the views borrows.

use std::path::PathBuf;

use model::graph::Row;
use model::repo::name_repositories;
use model::state::RepositoryState;
use views::activity::{ActivityState, RunningTask, StatusBarState};
use views::details::{DetailsState, SelectedDetails};
use views::graph::{
    BranchChip, Folds, GraphState, HeadName, HistoryState, MoreMenuState, Reading, ReferencesShown, RepositoryEntry,
    RepositoryPill, Selected, SyncCounts, TimelineMenu, ToolbarState, TraceShown,
};
use views::page::{Page, PageHostState, PageState};
use views::refs::ReferencesState;
use views::repo::{NoRepositoryState, OpenFolderQuestion, RepositoryPickerState};
use views::welcome::WelcomeState;
use views::{Content, TimelineState, WindowState};

use super::state::{Dialog, Shown, State};
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
    /// The sizes of the panes of the workspace.
    panes: WorkspacePanes,
    /// The moment the dates are said from, in seconds since 1970.
    now: i64,
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
            panes: workspace_panes(state),
            now,
        }
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
            drawer: state
                .is_activity_open
                .then_some(ActivityState { running: &self.running }),
            picker: (state.dialog == Some(Dialog::Picker)).then_some(RepositoryPickerState {
                repositories: &self.repositories,
            }),
            question: matches!(state.dialog, Some(Dialog::OpenFolder { .. })).then_some(OpenFolderQuestion {
                folder: &self.question_folder,
            }),
            notice: state.notice,
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
            more: MoreMenuState {
                can_refresh: state.workspace.is_some(),
                can_select_repository: state.workspace.is_some(),
                timeline: read.map(|_| timeline_menu(state)),
            },
            is_activity_open: state.is_activity_open,
            references: read.map(|_| ReferencesShown {
                is_shown: !self.panes.is_rail_hidden,
            }),
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
            None => Content::Timeline(self.timeline(state)),
        }
    }

    /// The timeline, the branch rail and the inspector of the repository shown.
    fn timeline<'a>(&'a self, state: &'a State) -> TimelineState<'a> {
        let shown = state.workspace.as_ref().map(|workspace| &workspace.shown);
        match shown {
            Some(Shown::Read(read)) => TimelineState::Repository {
                rail: ReferencesState {
                    repository: &read.state,
                    open: &state.kept_open,
                },
                graph: GraphState::Listed(Box::new(self.history(state, read.state.head.commit.is_none()))),
                inspector: DetailsState {
                    selected: selected_details(&state.timeline),
                },
                panes: self.panes.sizes,
                is_rail_hidden: self.panes.is_rail_hidden,
            },
            Some(Shown::NoRepository) => TimelineState::NoRepository(NoRepositoryState::NoRepository),
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
            switches: state.settings.switches,
            graph_width: self.panes.graph_width,
            now: self.now,
            found: &[],
            draft: timeline.draft,
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
