//! What the views are drawn from: the state of the window of [`views`], made from the store for each frame. The words
//! it needs that the store does not keep (the names of the repositories, the recent workspaces by their folders, what
//! is running) are made first, in a [`ViewData`] the state of the views borrows.

use std::path::PathBuf;

use model::repo::name_repositories;
use model::state::RepositoryState;
use views::activity::{ActivityState, RunningTask, StatusBarState};
use views::details::DetailsState;
use views::graph::{
    BranchChip, GraphState, HeadName, MoreMenuState, RepositoryEntry, RepositoryPill, SyncCounts, ToolbarState,
};
use views::page::{Page, PageHostState, PageState};
use views::refs::ReferencesState;
use views::repo::{NoRepositoryState, OpenFolderQuestion, RepositoryPickerState};
use views::welcome::WelcomeState;
use views::{Content, TimelineState, WindowState};

use super::state::{Dialog, Shown, State};
use crate::persisted::same_folder;

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
}

impl ViewData {
    /// The words of a state, with what is running.
    #[must_use]
    pub fn new(state: &State, running: Vec<RunningTask>) -> ViewData {
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
            },
            is_activity_open: state.is_activity_open,
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
            None => Content::Timeline(timeline(state)),
        }
    }
}

/// The timeline, the branch rail and the inspector of the repository shown.
fn timeline(state: &State) -> TimelineState<'_> {
    let shown = state.workspace.as_ref().map(|workspace| &workspace.shown);
    match shown {
        Some(Shown::Read(read)) => TimelineState::Repository {
            rail: ReferencesState {
                repository: &read.state,
                open: &state.kept_open,
            },
            graph: GraphState::Listed {
                rows: &read.commits.rows,
                has_no_commits: read.state.head.commit.is_none(),
            },
            inspector: DetailsState,
        },
        Some(Shown::NoRepository) => TimelineState::NoRepository(NoRepositoryState::NoRepository),
        Some(Shown::Looking | Shown::Reading { .. }) | None => TimelineState::NoRepository(NoRepositoryState::Loading),
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
