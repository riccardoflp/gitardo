//! What the store of a window is told ([`Message`]) and what it asks of the outside ([`Effect`]).

use std::path::PathBuf;
use std::time::Duration;

use git::read::details::CommitRequest;
use git::read::graph::{CommitPage, CommitsRequest, Parents};
use model::CommitId;
use model::commit_page::PatchLines;
use model::details::CommitDetail;
use model::filter::BranchesShown;
use model::graph::PageSize;
use model::graph::Row;
use model::repo::{Change, Repository};
use model::state::RepositoryState;
use views::Intent;

use crate::persisted::Persisted;
use crate::settings::{Settings, SettingsFiles};

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
    /// A job that looked for the repositories of folders is done.
    RepositoriesFound {
        /// The job.
        job: JobId,
        /// What it found.
        found: Found,
    },
    /// A job that read a repository is done.
    RepositoryRead {
        /// The job.
        job: JobId,
        /// The folder read.
        folder: PathBuf,
        /// What was read, or what git said when the read failed.
        result: Result<Box<(RepositoryState, CommitPage)>, String>,
        /// When it was read, in seconds since 1970: the lines between days and months are counted from it.
        read_at: i64,
    },
    /// A job that read the first page of another history of the timeline is done (`filter.show-only`).
    TimelineRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<CommitPage, String>,
    },
    /// A job that read the next pages of the timeline is done (`graph.load-more`).
    CommitsRead {
        /// The job.
        job: JobId,
        /// What was read, or what git said when the read failed.
        result: Result<MorePages, String>,
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
    /// The watcher of the repository saw changes (`repo.watch-changes`), at a moment counted from when the window
    /// opened.
    Changed {
        /// What changed.
        changes: Vec<Change>,
        /// When.
        at: Duration,
    },
    /// The moment an [`Effect::WakeAt`] asked for came, or later.
    Wake {
        /// The moment, counted from when the window opened.
        at: Duration,
    },
    /// The window was minimized, or shown again: nothing of the repository is in sight while it is minimized.
    Minimized(bool),
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
    /// A task of the Activity drawer started, printed or ended (`worktree-setup.command-steps`).
    Task(super::tasks::TaskMessage),
    /// The browser of the system could not open an address: what the system said.
    UrlNotOpened(String),
    /// The person asked to close the window, whose inside had this size in logical pixels.
    CloseRequested {
        /// The size of the inside of the window.
        window_size: Option<(f32, f32)>,
    },
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

/// The next pages of the timeline, as a worker sends them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MorePages {
    /// The commits known are read again as they were: only the rows after them are sent.
    Appended {
        /// The rows after those known.
        rows: Vec<Row>,
        /// Whether the history has more.
        has_more: bool,
    },
    /// The history changed under the commits known: every row read.
    Replaced(CommitPage),
}

/// The commits a window has of a timeline, as a worker checks them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Known {
    /// How many rows were read, the stashes among them.
    pub count: usize,
    /// How many of them are commits.
    pub commits: usize,
    /// The last commit read: the read of the next pages starts from it again, to tell whether the history is the
    /// same above it.
    pub last: Option<model::CommitId>,
}

/// How the timeline of a repository is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineRequest {
    /// How many commits a page has.
    pub page_size: PageSize,
    /// How many pages are read, from the newest commit.
    pub pages: usize,
    /// Which parents are followed (`graph.firstParent`).
    pub parents: Parents,
    /// Whether the stashes are placed among the commits (`graph.showStashes`).
    pub shows_stashes: bool,
    /// The branches shown, as chosen: the worker checks them against the references it reads (`filter.remember`).
    pub shown: Option<BranchesShown>,
    /// What narrows the history further: the arguments of a search shown as a list (`search.list-mode`).
    pub narrowing: Vec<std::ffi::OsString>,
    /// The commit whose history alone is listed, in place of the branches shown: a commit gone to that is on no branch
    /// or tag (`search.go-to-hash`).
    pub alone: Option<model::CommitId>,
}

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
    /// Reads a repository: its state (`repo.read-state`), then the first page of its commits (`graph.read-commits`).
    ReadRepository {
        /// The folder of the repository.
        folder: PathBuf,
        /// How the timeline is read.
        request: TimelineRequest,
    },
    /// Reads the first page of the timeline for the branches shown, and nothing else of the repository: the timeline
    /// narrowed, or shown whole again (`filter.show-only`, `filter.show-all`).
    ReadTimeline {
        /// The folder of the repository.
        folder: PathBuf,
        /// What is read.
        request: CommitsRequest,
    },
    /// Reads the pages of the timeline from the first again, one more than are loaded, and sends only the rows after
    /// those known when they are the same (`graph.load-more`).
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

/// What the store asks of the outside, carried out after the reduce, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Runs a job on a worker.
    Start(Job),
    /// Cancels a job that is not wanted any more: the git it runs is stopped.
    Cancel(JobId),
    /// Asks the AI provider a question, on a thread of its own (`ai.resolve-conflicts`, `commit.draft-message`).
    Ask(Box<crate::work::ai::AiQuestion>),
    /// Cancels questions to the AI provider under way: nothing more comes of them.
    StopAsking(Vec<crate::work::ai::AskId>),
    /// Opens the system's folder dialog, whose answer comes back as [`Message::FolderChosen`].
    ChooseFolder,
    /// Writes what is kept between runs.
    WritePersisted(Persisted),
    /// Sets a value in a settings file, keeping the rest of its text, on a thread of its own: a switch of the timeline
    /// in the user's file.
    WriteSetting {
        /// The file.
        file: PathBuf,
        /// The key.
        key: &'static str,
        /// The value.
        value: json::Value,
    },
    /// Watches these settings files, in place of those watched before.
    WatchSettings(SettingsFiles),
    /// Draws the window with the theme and the fonts of the settings in force.
    ApplySettings,
    /// Watches this working tree and its git folder for changes, in place of the one watched before; none stops
    /// watching (`repo.watch-changes`).
    WatchRepository(Option<PathBuf>),
    /// Sends [`Message::Wake`] at this moment, counted from when the window opened, in place of the wake asked for
    /// before (`repo.schedule-reads`).
    WakeAt(Duration),
    /// Runs an operation of the catalog in a repository, on the thread of the operations; the window sets the time of
    /// its context.
    RunFeature {
        /// The repository.
        folder: PathBuf,
        /// The operation.
        feature: model::contract::FeatureId,
        /// The answers it is given before it asks: none, or what the entry chosen names.
        preset: model::contract::Answers,
        /// What it is given, boxed for its settings.
        context: Box<model::contract::Context>,
    },
    /// Gives the operation running what the user answered.
    ReplyToFeature(super::feature::Reply),
    /// Puts text on the clipboard of the system: the URL of a remote (`remote.manage`).
    CopyText(String),
    /// Opens a folder in a new window of Gitardo Desktop, as a repository of its own
    /// (`submodule.open-in-new-window`, `worktree.open-in-new-window`).
    OpenWindow(PathBuf),
    /// Opens the system's dialog to choose a file to open, or where to save one, for the question of the operation
    /// running; the path chosen, or none, is its answer.
    ChooseFile {
        /// The title of the dialog.
        title: String,
        /// Where it starts and which files it shows.
        question: model::contract::FileQuestion,
        /// Whether it chooses where to save, rather than a file to open.
        is_saving: bool,
    },
    /// Looks for the repository on GitHub, or reads its pull requests, on a thread of its own
    /// (`github.read-pull-requests`).
    GitHub(super::github::GitHubJob),
    /// Runs, answers or stops the tasks of the Activity drawer (`worktree-setup.command-steps`).
    Tasks(super::tasks::TaskEffect),
    /// Opens an address in the browser of the system: a pull request on GitHub (`github.open-pull-request`).
    OpenUrl(String),
    /// Closes the window and ends the program.
    Quit,
}
