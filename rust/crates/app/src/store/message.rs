//! What the store of a window is told ([`Message`]), and the jobs it gives the workers.

use std::path::PathBuf;
use std::time::Duration;

use git::read::details::CommitRequest;
use git::read::graph::{CommitPage, CommitsRequest};
use model::CommitId;
use model::commit_page::PatchLines;
use model::details::CommitDetail;
use model::repo::{Change, Repository};
use model::state::RepositoryState;
use views::Intent;

pub use super::effect::Effect;
pub use super::timeline_message::{Known, MorePages};
use crate::settings::Settings;

/// What the store of a window is told: what the user wants, and what the outside answers.
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// The window opened, on the folders of the command line, or on none.
    Started {
        /// The folders of the command line: `gitardo <folder>...`.
        folders: Vec<PathBuf>,
    },
    /// What a view returned.
    Intent(Intent),
    /// The answer of the system's folder dialog: none when it was cancelled.
    FolderChosen(Option<PathBuf>),
    /// The answer of the system's dialog for a workspace file of VS Code: the file and its text, or what kept it from
    /// being read; none when it was cancelled.
    WorkspaceFileChosen(Option<(PathBuf, Result<String, String>)>),
    /// A job that looked for the repositories of folders is done.
    RepositoriesFound {
        /// The job.
        job: JobId,
        /// What it found.
        found: Found,
    },
    /// A job that read a repository has read its state and the first page of its commits; the next pages, when more
    /// follow, come as [`Message::PagesRead`].
    RepositoryRead {
        /// The job.
        job: JobId,
        /// The folder read.
        folder: PathBuf,
        /// What was read, with what was counted in the other worktrees, or what git said when the read failed: no page
        /// when the history is the one read before, whose rows the timeline keeps.
        result: Result<
            Box<(
                RepositoryState,
                Option<CommitPage>,
                git::read::worktree::CountedWorktrees,
            )>,
            String,
        >,
        /// When it was read, in seconds since 1970: the lines between days and months are counted from it.
        read_at: i64,
    },
    /// A job that read another history of the timeline has its first page (`filter.show-only`); the next pages, when
    /// more follow, come as [`Message::PagesRead`].
    TimelineRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<CommitPage, String>,
    },
    /// A job that read the next pages of the timeline has its first page (`graph.load-more`).
    CommitsRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<MorePages, String>,
    },
    /// A read of the timeline whose first page came has another, as git prints it (`graph.read-commits`).
    PagesRead {
        /// The job.
        job: JobId,
        /// The page, or what git said when the read failed.
        result: Result<CommitPage, String>,
    },
    /// A read of the search is done.
    Searched {
        /// The job.
        job: JobId,
        /// What was read.
        result: crate::store::search::SearchResult,
    },
    /// A job that read the details of the row selected is done (`details.read-commit`).
    DetailsRead {
        /// The job.
        job: JobId,
        /// What was read, none for a commit that is not in the repository, or what git said when the read failed.
        result: Result<Option<Box<CommitDetail>>, String>,
    },
    /// A job that read the commit of a page and its lines is done (`details.read-commit`, `commit-page.read-patch`).
    CommitPageRead {
        /// The job.
        job: JobId,
        /// What was read, none for a commit that is not in the repository, or what git said when the read failed.
        result: Result<Option<Box<(CommitDetail, PatchLines)>>, String>,
    },
    /// A job that read and computed the diff of a file of a Diff page is done (`diff.page`).
    DiffRead {
        /// The job.
        job: JobId,
        /// What it came to.
        result: crate::work::diff::DiffRead,
    },
    /// A job that read a file in conflict and computed its regions is done (`merge-view.page`).
    MergeRead {
        /// The job.
        job: JobId,
        /// What it came to.
        result: crate::work::merge_view::MergeRead,
    },
    /// A job that read the file of a File page is done (`file-view.page`).
    FileRead {
        /// The job.
        job: JobId,
        /// The bytes, none when the file is not there, or what the system or git said.
        result: Result<Option<Vec<u8>>, String>,
    },
    /// A job that read an image of the preview of a Markdown file is done (`file-view.markdown-preview`).
    ImageRead {
        /// The job.
        job: JobId,
        /// The image, none when it is not there or could not be read as an image.
        image: Option<render::Image>,
    },
    /// A job that drew a diagram of Mermaid of the preview of a Markdown file is done (`file-view.mermaid`).
    DiagramDrawn {
        /// The job.
        job: JobId,
        /// The diagram, or why it is not drawn.
        diagram: Result<views::file_view::DrawnDiagram, views::file_view::DiagramFailure>,
    },
    /// A job that read a comparison page is done (`compare.read-sides`).
    ComparisonRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<Box<crate::work::compare::ComparisonRead>, String>,
    },
    /// A job that read the files of the comparison in the inspector is done (`compare.read-files`).
    InPlaceRead {
        /// The job.
        job: JobId,
        /// The files that differ, or what git said when the read failed.
        result: Result<Vec<model::state::FileChange>, String>,
    },
    /// A job that counted the conflicts of the files in conflict is done (`conflicts.banner`).
    ConflictsCounted {
        /// The job.
        job: JobId,
        /// How many conflicts there are and how many need no choice, or what git said when the read failed.
        result: Result<model::conflicts::RuleCounts, String>,
    },
    /// A job that read the files in conflict for their review is done (`review.page`).
    ReviewRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<Box<crate::work::review::ReviewReadout>, String>,
    },
    /// A job that asked git which of the files with edits not saved it ignores is done (`changes.unsaved`).
    IgnoredRead {
        /// The job.
        job: JobId,
        /// The paths git ignores, or what git said when the read failed.
        result: Result<Vec<String>, String>,
    },
    /// A job that listed the files of the working tree for the Files page is done.
    FilesRead {
        /// The job.
        job: JobId,
        /// The paths, or what git said when the read failed.
        result: Result<Vec<String>, String>,
    },
    /// A read of the Search page says something: the files listed, the names ranked, or more lines of text found
    /// (`file-search.page`).
    FileSearch {
        /// The job.
        job: JobId,
        /// What it says.
        answer: crate::work::file_search::SearchAnswer,
    },
    /// A job that read the settings of new worktrees, and what git ignores, for their page is done
    /// (`worktree-setup.page`).
    NewWorktreesRead {
        /// The job.
        job: JobId,
        /// What was read.
        result: Box<crate::work::new_worktrees::SetupReadout>,
    },
    /// A job that read the blame of a File page is done (`blame.current-line`, `blame.file`).
    BlameRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said, or that it was cancelled.
        result: Result<crate::work::blame::BlameAnswer, String>,
    },
    /// The settings files changed and were read again.
    SettingsRead(Box<Settings>),
    /// The watcher of the repository saw changes (`repo.watch-changes`), at a moment since the window opened.
    Changed {
        /// What changed.
        changes: Vec<Change>,
        /// When.
        at: Duration,
    },
    /// The watcher of a tab not shown saw changes (`repo.watch-hidden`), at a moment since the window opened.
    TabChanged {
        /// The working tree watched, as [`Effect::WatchTabs`] named it.
        folder: PathBuf,
        /// What changed.
        changes: Vec<Change>,
        /// When.
        at: Duration,
    },
    /// What the reads in the background of the folders not shown found (`repo.worktree-summary`).
    SummariesRead(Vec<crate::work::summaries::SummaryRead>),
    /// The moment an [`Effect::WakeAt`] asked for came, or later.
    Wake {
        /// The moment, counted from when the window opened.
        at: Duration,
    },
    /// The window was minimized, or shown again: nothing of the repository is in sight while it is minimized.
    Minimized(bool),
    /// Who draws the title bar of the window, and whether it is maximized, since this changed
    /// (`window.title-bar`).
    Caption(views::window::CaptionState),
    /// A read of the repository ended, after so long: its line in the Command log at `log.level` `debug`.
    ReadTook {
        /// The words of the line.
        line: String,
        /// How long it took.
        took: Duration,
        /// When it started, in seconds since 1970.
        time: model::Timestamp,
    },
    /// The operation running asks a question.
    FeatureAsks(Box<model::contract::Question>),
    /// The operation running shows its plan before it runs it (`preview.confirm`).
    FeatureConfirms(Box<model::contract::Plan>),
    /// The operation running ended, with the lines its engine logged.
    FeatureEnded {
        /// How it ended.
        ended: super::feature::Ended,
        /// The lines of the Command log.
        log: Vec<engine::LogEntry>,
    },
    /// The text of the answer of a question to the AI provider, so far (`ai.resolve-conflicts`).
    AiPiece {
        /// The question.
        ask: crate::work::ai::AskId,
        /// The text so far.
        text: String,
    },
    /// A question to the AI provider is answered, or failed.
    AiAnswered {
        /// The question.
        ask: crate::work::ai::AskId,
        /// What it came to.
        result: crate::work::ai::AiResult,
    },
    /// A job of the pull requests of GitHub is done (`github.read-pull-requests`).
    GitHub(super::github::GitHubMessage),
    /// Lines of the Command log for the commands of a new worktree, or the end of the steps after them
    /// (`worktree-setup.command-steps`).
    Commands(super::tasks_tab::CommandsMessage),
    /// A terminal of the bottom panel started, printed or ended (`terminal.view`).
    Terminal(super::terminals::TerminalMessage),
    /// What the window's task runs tell of the tasks of the Tasks tab (`tasks.view`).
    TaskRun(super::tasks_tab::TaskRunMessage),
    /// The browser of the system could not open an address: what the system said.
    UrlNotOpened(String),
    /// The person asked to close the window, which was there, so large and maximized or not.
    CloseRequested(crate::persisted::WindowPlace),
}

/// The repositories found in some folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    /// The repositories, in the order they were found; none when the folders hold none.
    Repositories(Vec<Repository>),
    /// None of the folders is there any more.
    AllGone,
    /// Git could not be run: what it said.
    Failed(String),
}

/// How the timeline of a repository is read: the engine's, which turns it into git's arguments.
pub use engine::timeline::TimelineRequest;

/// The number of a job, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

/// What a job does, on a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobKind {
    /// Looks for the repositories of folders (`repo.read-repositories`).
    FindRepositories {
        /// The folders.
        folders: Vec<PathBuf>,
    },
    /// Reads a repository: its state (`repo.read-state`), then its commits, a page at a time (`graph.read-commits`).
    ReadRepository {
        /// The folder of the repository.
        folder: PathBuf,
        /// How the timeline is read.
        request: TimelineRequest,
        /// The state of the read before, when the timeline may keep its rows: the commits are not read again when the
        /// history is the same (`repo.schedule-reads`).
        kept: Option<std::sync::Arc<RepositoryState>>,
        /// What the read before counted in the other worktrees; empty to count them all again (M15).
        counted: git::read::worktree::CountedWorktrees,
    },
    /// Reads the timeline for the branches shown, and nothing else of the repository: the timeline narrowed, or shown
    /// whole again (`filter.show-only`, `filter.show-all`).
    ReadTimeline {
        /// The folder of the repository.
        folder: PathBuf,
        /// What is read.
        request: CommitsRequest,
    },
    /// Reads the next commits of the timeline from the last one known, past those one read lists, and sends only the
    /// rows after it when the history is the same up to it; every row again otherwise (`graph.load-more`).
    ReadCommits {
        /// The folder of the repository.
        folder: PathBuf,
        /// What is read.
        request: CommitsRequest,
        /// The rows the window has.
        known: Known,
    },
    /// A read of the search: the commits it finds, the commit of a name, or where a commit is in the history.
    Search(crate::store::search::SearchJob),
    /// Reads the details of the row selected (`details.read-commit`).
    ReadDetails {
        /// The folder of the repository.
        folder: PathBuf,
        /// The commit, or the uncommitted changes.
        request: CommitRequest,
    },
    /// Reads the commit of a page, then the lines it changed (`commit-page.read-patch`).
    ReadCommitPage {
        /// The folder of the repository.
        folder: PathBuf,
        /// The commit.
        commit: CommitId,
    },
    /// Reads the two versions of a file of a Diff page and computes their diff (`diff.read-sides`, `diff.page`).
    ReadDiff {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        job: crate::work::diff::DiffJob,
    },
    /// Reads a file in conflict of a Merge page and computes its regions (`merge-view.read-sides`, `merge-view.page`).
    ReadMerge {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        job: crate::work::merge_view::MergeJob,
    },
    /// Reads the file of a File page, from the disk or at a revision (`file-view.page`).
    ReadFile {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        job: crate::work::file_view::FileJob,
    },
    /// Reads an image of the repository the preview of a Markdown file shows, from the disk or at a revision
    /// (`file-view.markdown-preview`).
    ReadImage {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// The image read.
        job: crate::work::file_view::FileJob,
    },
    /// Draws a diagram of Mermaid of the preview of a Markdown file in a theme (`file-view.mermaid`).
    DrawDiagram {
        /// The top folder of the working tree of the file.
        folder: PathBuf,
        /// The diagram drawn.
        job: crate::work::diagram::DiagramJob,
    },
    /// Reads a comparison page: its followed sides where they are now, then what differs (`compare.read-sides`).
    ReadComparison {
        /// The folder of the repository.
        folder: PathBuf,
        /// The comparison.
        target: Box<model::compare::ComparisonTarget>,
    },
    /// Reads the files of the comparison in the inspector (`compare.read-files`).
    ReadInPlace {
        /// The folder of the repository.
        folder: PathBuf,
        /// The two commits.
        request: git::read::compare::FilesRequest,
    },
    /// Reads the files in conflict and counts the conflicts the rules resolve (`conflicts.banner`).
    CountConflicts {
        /// The top folder of the working tree.
        folder: PathBuf,
    },
    /// Reads the files in conflict for their review (`review.page`).
    ReadReview {
        /// The folder of the repository.
        folder: PathBuf,
        /// Which files.
        request: crate::work::review::ReviewRequest,
    },
    /// Asks git which of the files with edits not saved it ignores (`changes.unsaved`).
    ReadIgnored {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// The files, from it.
        paths: Vec<String>,
    },
    /// Lists the files of the working tree for the Files page.
    ReadFiles {
        /// The top folder of the working tree.
        folder: PathBuf,
    },
    /// Lists the files of the working tree for the Search page, or searches them (`file-search.page`).
    FileSearch {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        job: crate::work::file_search::SearchJob,
    },
    /// Reads the settings of new worktrees, and what git ignores when the page looks, for their page
    /// (`worktree-setup.page`).
    ReadNewWorktrees {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        request: crate::work::new_worktrees::SetupRequest,
    },
    /// Reads the blame of the line under the cursor of a File page, or of its file (`blame.current-line`,
    /// `blame.file`).
    ReadBlame {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        job: crate::work::blame::BlameJob,
    },
}

/// A job for a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// Its number, which its result carries back.
    pub id: JobId,
    /// What it does.
    pub kind: JobKind,
}
