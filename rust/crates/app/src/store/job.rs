//! A job for a worker, its number, and what it does.

use std::path::PathBuf;

use git::read::details::CommitRequest;
use git::read::graph::CommitsRequest;
use model::CommitId;
use model::state::RepositoryState;

use super::message::{Known, TimelineRequest};

/// The number of a job, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

/// A job for a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// Its number, which its result carries back.
    pub id: JobId,
    /// What it does.
    pub kind: JobKind,
}

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
    /// Reads the authors of the history for the picker of the co-authors (`commit.read-authors`).
    ReadAuthors {
        /// The folder of the repository.
        folder: PathBuf,
        /// Which history.
        request: git::read::commit::AuthorsRequest,
    },
    /// Asks a remote for its tags, and reads those of the repository, for the marks of the tags it lacks
    /// (`tag.read-remote-tags`, `tag.remote-marks`).
    ReadRemoteTags {
        /// The folder of the repository.
        folder: PathBuf,
        /// Which remote.
        request: git::read::tag::RemoteTagsRequest,
    },
    /// Reads the signature of the commit of the inspector or of a page (`details.read-signature`).
    ReadSignature {
        /// The folder of the repository.
        folder: PathBuf,
        /// The commit.
        request: git::read::details::SignatureRequest,
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
    /// Reads the two versions of an image file of the page of a commit and draws them (`commit-page.images`).
    ReadImages {
        /// The folder of the repository.
        folder: PathBuf,
        /// The two versions.
        request: git::read::diff::DiffRequest,
    },
    /// Draws the picture of the bytes of an image file of a File page (`file-view.image`).
    DrawImage {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// The path of the file, which tells an SVG.
        path: String,
        /// Its bytes.
        bytes: Vec<u8>,
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
    /// Reads Git LFS and the files it stores, in the top folder of a working tree, for the Large files page, and the
    /// locks of the server when asked (`lfs.read-locks`).
    ReadLargeFiles(PathBuf, bool),
    /// Reads the settings of new worktrees, and what git ignores when the page looks, for their page
    /// (`worktree-setup.page`).
    ReadNewWorktrees {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// What is read.
        request: crate::work::new_worktrees::SetupRequest,
    },
    /// Reads the pairs of the page of what a rewrite changed (`range-diff.read-pairs`).
    ReadRangeDiff {
        /// The top folder of the working tree.
        folder: PathBuf,
        /// The two lines compared.
        request: git::read::range_diff::RangeDiffRequest,
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
