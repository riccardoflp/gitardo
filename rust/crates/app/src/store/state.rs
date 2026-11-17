//! What the store of a window holds: the workspace and the repository shown in it as last read, the pages open, the
//! dialogs, the settings in force and what is kept between runs.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use model::repo::Repository;
use model::state::RepositoryState;
use views::Notice;
use views::page::Page;

use super::message::{JobId, JobKind};
use crate::persisted::Persisted;
use crate::settings::Settings;

/// Everything a window shows, owned by the thread of the window and changed only by [`super::reduce`].
#[derive(Clone, Debug, PartialEq, Default)]
pub struct State {
    /// The folders open and the repository shown; none before a folder is opened.
    pub workspace: Option<Workspace>,
    /// A workspace being opened, until its repositories are found.
    pub opening: Option<Opening>,
    /// What is kept between runs: the recent workspaces, the steps of the Welcome page done.
    pub persisted: Persisted,
    /// The settings in force.
    pub settings: Settings,
    /// The user's settings file, which is watched with the one of the repository shown; none when the system does
    /// not say where the user's folders are.
    pub user_settings_file: Option<PathBuf>,
    /// The pages open, and which one is shown.
    pub pages: Pages,
    /// The dialog open over the window.
    pub dialog: Option<Dialog>,
    /// A sentence said once, until it is put away.
    pub notice: Option<Notice>,
    /// The branch rail and the branch filter.
    pub rail: super::rail::Rail,
    /// Whether the Activity drawer is open.
    pub is_activity_open: bool,
    /// Whether the side column is opened over the timeline, in the narrow layout (`repo.side-column`).
    pub is_side_column_over: bool,
    /// Whether an operation runs: choosing another repository waits for it (`repo.select`).
    pub is_operation_running: bool,
    /// The operation running, and what was said once the last one ended.
    pub operations: super::feature::Operations,
    /// The jobs started and not finished, by their number: a result whose job is not here any more is not wanted.
    pub jobs: BTreeMap<JobId, JobKind>,
    /// The number of the next job.
    pub next_job: u64,
    /// The number the next read of a repository gets, so that the views can tell a new read from the last one.
    pub next_generation: u64,
    /// What the last read that failed said, for the Command log once it exists.
    pub last_error: Option<String>,
    /// The timeline of the repository shown.
    pub timeline: super::timeline::Timeline,
    /// The search of the timeline, **Go to commit** and Quick search.
    pub search: super::search::Search,
    /// The inspector of the row selected.
    pub inspector: super::inspector::Inspector,
    /// The pages of commits open.
    pub commit_pages: super::commit_page::CommitPages,
    /// The Diff pages open, or waiting for their first read (`diff.page`).
    pub diff_pages: super::diff_page::DiffPages,
    /// The Merge pages open, or waiting for their first read (`merge-view.page`).
    pub merge_pages: super::merge_page::MergePages,
    /// The File pages open (`file-view.page`).
    pub file_pages: super::file_page::FilePages,
    /// The comparison pages, the comparisons opened before and the comparison in the inspector.
    pub compares: super::compare::Compares,
    /// The review of the conflicts (`review.page`).
    pub reviews: super::review::Reviews,
    /// The reads of the repository shown that changes ask for, and the ones the window starts itself
    /// (`repo.schedule-reads`).
    pub reads: super::reads::Reads,
    /// The Command log of the window (`log.record`, `log.show`).
    pub log: engine::CommandLog,
    /// The bottom panel (`panel.bottom`).
    pub panel: super::panel::Panel,
    /// The terminals of the bottom panel, of each repository tab (`terminal.view`).
    pub terminals: super::terminals::Terminals,
    /// The file the settings page writes changes in (`settings.edit`).
    pub settings_target: views::settings::SettingsTarget,
    /// The Changes page and the commit box (`changes.pane`).
    pub changes: super::changes::Changes,
    /// The interactive rebase page, while it is open (`rebase.todo-page`).
    pub rebase_page: Option<views::rebase::TodoPage>,
    /// The conflicts the rules resolve, counted in the repository shown (`conflicts.banner`).
    pub conflicts: super::conflicts::Conflicts,
    /// The Files page.
    pub files_page: super::files_page::FilesPage,
    /// The page that chooses what new worktrees get (`worktree-setup.page`).
    pub new_worktrees: super::new_worktrees::NewWorktreesPage,
    /// The blame of the File pages (`blame.current-line`, `blame.file`).
    pub blames: super::blame::Blames,
    /// Another worktree shown in this window, and what the operations of worktrees are told of the window.
    pub worktrees: super::worktrees::Worktrees,
    /// The pull requests of GitHub of the repository shown, and when they are read again.
    pub github: super::github::GitHub,
    /// The tabs of the repositories open, and what those not shown keep (`repo.tabs`, `repo.keep-open`).
    pub tabs: super::tabs::Tabs,
    /// The Tasks of the Activity drawer: the commands of new worktrees (`worktree-setup.command-steps`).
    pub tasks: super::tasks::Tasks,
}

/// The folders of a window and the repositories found in them.
#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    /// Its folders, in their order.
    pub folders: Vec<PathBuf>,
    /// The repositories found in them, in the order they were found; none while they are looked for.
    pub repositories: Option<Vec<Repository>>,
    /// The repository shown, and how far it is read.
    pub shown: Shown,
}

/// The repository shown in a workspace.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// The repositories are looked for.
    Looking,
    /// A repository is read for the first time since it was chosen.
    Reading {
        /// Its folder.
        folder: PathBuf,
    },
    /// A repository as last read; it may be read again meanwhile.
    Read(RepositoryRead),
    /// The workspace has no repository, or the one shown could not be read.
    NoRepository,
}

/// A repository as read, immutable: a read makes a new one with a new generation.
#[derive(Clone, Debug, PartialEq)]
pub struct RepositoryRead {
    /// Its folder, as found among the repositories of the workspace.
    pub folder: PathBuf,
    /// Its references, its `HEAD` and the rest of its state; its commits are in the [`super::timeline::Timeline`].
    pub state: Arc<RepositoryState>,
    /// Which read this is.
    pub generation: u64,
}

/// A workspace being opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    /// The job that looks for its repositories.
    pub job: JobId,
    /// Its folders.
    pub folders: Vec<PathBuf>,
    /// The repository to show when it is among those found: the one shown in it last, or the one shown before a
    /// folder was added.
    pub remembered: Option<PathBuf>,
    /// The place of the workspace among the recent ones, when it is one of them: one whose folders are all gone is
    /// refused and taken out of the list.
    pub recent: Option<usize>,
    /// Whether it is the workspace shown, looked at again: the repository shown is read again even when it is the
    /// one chosen.
    pub is_refresh: bool,
    /// Whether its folders are those of the workspace shown with one added: its tabs stay, and the repositories of the
    /// folder added are offered (`repo.choose-tabs`).
    pub is_added: bool,
    /// The workspace file of VS Code its folders come from (`repo.open-code-workspace`).
    pub file: Option<PathBuf>,
    /// The page the window showed when it had no workspace and showed this one at once, given back when the choice of
    /// the tabs is cancelled.
    pub page_before: Option<Option<usize>>,
}

/// Folders chosen to open, from a folder or from a workspace file of VS Code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    /// The folders.
    pub folders: Vec<PathBuf>,
    /// The workspace file of VS Code that lists them, when they come from one.
    pub file: Option<PathBuf>,
}

impl Opened {
    /// What the question asked about them names: the workspace file, or the first folder.
    #[must_use]
    pub fn name(&self) -> String {
        self.file
            .as_ref()
            .or(self.folders.first())
            .map(|path| {
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                )
            })
            .unwrap_or_default()
    }
}

/// The pages open.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pages {
    /// The pages open, in the order they were opened.
    pub open: Vec<Page>,
    /// The page shown; none when the timeline is.
    pub shown: Option<usize>,
}

/// A dialog over the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dialog {
    /// The picker of the repository shown (`repo.select`).
    Picker,
    /// Whether the folders chosen replace the workspace or are added to it (`repo.open-folder`,
    /// `repo.open-code-workspace`).
    OpenFolder {
        /// The folders chosen.
        opened: Opened,
    },
    /// The picker of the themes (`settings.theme`).
    ThemePicker,
    /// The picker of the repositories to open in tabs, all checked (`repo.choose-tabs`).
    ChooseTabs(Box<super::tabs::Choosing>),
    /// The picker of a repository of the workspace to open in a tab (`repo.open-tab`).
    OpenTab,
}

impl State {
    /// A window that has not started, with what was kept from the runs before and the settings read.
    #[must_use]
    pub fn new(persisted: Persisted, settings: Settings, user_settings_file: Option<PathBuf>) -> State {
        State {
            log: engine::CommandLog::new(settings.log_level),
            persisted,
            settings,
            user_settings_file,
            ..State::default()
        }
    }

    /// The repository shown as last read, if one is.
    #[must_use]
    pub fn shown_read(&self) -> Option<&RepositoryRead> {
        match &self.workspace.as_ref()?.shown {
            Shown::Read(read) => Some(read),
            _ => None,
        }
    }

    /// The folder of the repository shown or being read for the first time.
    #[must_use]
    pub fn shown_folder(&self) -> Option<&PathBuf> {
        match &self.workspace.as_ref()?.shown {
            Shown::Read(read) => Some(&read.folder),
            Shown::Reading { folder } => Some(folder),
            Shown::Looking | Shown::NoRepository => None,
        }
    }

    /// The repositories found in the workspace; none while they are looked for or with no workspace.
    #[must_use]
    pub fn repositories(&self) -> &[Repository] {
        self.workspace
            .as_ref()
            .and_then(|workspace| workspace.repositories.as_deref())
            .unwrap_or_default()
    }

    /// The page shown, if a page is.
    #[must_use]
    pub fn page_shown(&self) -> Option<Page> {
        self.pages.shown.and_then(|index| self.pages.open.get(index)).copied()
    }

    /// The File page in front: the page shown, or the file shown beside the tree of the Files page
    /// (`file-view.beside-files`).
    #[must_use]
    pub fn file_in_front(&self) -> Option<views::file_view::FilePageId> {
        match self.page_shown()? {
            Page::File(id) => Some(id),
            Page::Files => self.files_page.shown_file,
            // The other pages show no file of their own.
            _ => None,
        }
    }
}
