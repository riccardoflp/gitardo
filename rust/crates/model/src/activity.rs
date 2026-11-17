//! The words the activity indicator of the header and the Activity tab use for each thing that takes time
//! (`activity.describe-task`).
//!
//! The card gives this derivation its words, as `graph.date-separators` does (open question Q10): they are here rather
//! than in the views.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::repo::{FilesChange, ReadCause};

/// Something that takes time, as the activity indicator and the Activity tab know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// What it is.
    pub kind: TaskKind,
    /// When it started.
    pub started: Instant,
}

/// What a task is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    /// A read of a repository.
    Read {
        /// The name of the repository, as `repo.name-repositories` gives it.
        name: String,
        /// Whether a search in the changes (`git log -S` or `-G`) narrows the timeline.
        is_search: bool,
        /// Why it is read; none when nothing says.
        cause: Option<ReadCause>,
        /// How many reads in a row it is.
        in_a_row: u32,
    },
    /// An operation, by the title of its plan, as the preview shows it.
    Operation {
        /// The title.
        title: String,
    },
    /// A comparison of two sides.
    Comparison {
        /// The side compared from.
        from: String,
        /// The side compared to.
        to: String,
    },
    /// A patch being made.
    Patch {
        /// The commits, as the patch names them.
        commits: String,
    },
    /// The pull requests read from GitHub.
    PullRequests,
}

/// What is said of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskText {
    /// Its title: `Reading app`.
    pub title: String,
    /// Why it runs, when that is said: `Because src/a.rs changed.`
    pub detail: Option<String>,
    /// How long it has run: `12 s`, `1 min 5 s`.
    pub elapsed: String,
}

/// The title, the detail and the time of a task at a moment.
///
/// Feature: activity.describe-task
#[must_use]
pub fn describe_task(task: &Task, now: Instant) -> TaskText {
    let elapsed = elapsed_words(now.saturating_duration_since(task.started));
    let (title, detail) = match &task.kind {
        TaskKind::Read {
            name,
            is_search,
            cause,
            in_a_row,
        } => {
            let title = if *is_search {
                format!("Searching {name}")
            } else {
                format!("Reading {name}")
            };
            (title, read_detail(cause.as_ref(), *in_a_row))
        }
        TaskKind::Operation { title } => (title.clone(), None),
        TaskKind::Comparison { from, to } => (format!("Comparing {from} \u{2194} {to}"), None),
        TaskKind::Patch { commits } => (format!("Making a patch of {commits}"), None),
        TaskKind::PullRequests => (String::from("Reading the pull requests from GitHub"), None),
    };
    TaskText { title, detail, elapsed }
}

/// The title of things of the same words that run one after the other, on the one row that counts them, with how
/// many they are from the second on: `Reading app ×4`.
#[must_use]
pub fn counted(title: &str, count: u32) -> String {
    if count < 2 {
        title.to_owned()
    } else {
        format!("{title} \u{d7}{count}")
    }
}

/// How long something ran, in whole seconds, `12 s`, and from a minute on `1 min 5 s`.
#[must_use]
pub fn elapsed_words(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        format!("{seconds} s")
    } else {
        format!("{} min {} s", seconds / 60, seconds % 60)
    }
}

/// Why a read runs: the change that asked for it, after how many reads in a row it is from the second on.
fn read_detail(cause: Option<&ReadCause>, in_a_row: u32) -> Option<String> {
    let because = cause.map(|cause| format!("Because {}.", cause_words(cause)));
    let repeated =
        (in_a_row >= 2).then(|| format!("Read {in_a_row} times in a row: something changed during each read."));
    match (repeated, because) {
        (Some(repeated), Some(because)) => Some(format!("{repeated} {because}")),
        (repeated, because) => repeated.or(because),
    }
}

/// What asked for a read, in the words that follow **Because**.
#[must_use]
pub fn cause_words(cause: &ReadCause) -> String {
    match cause {
        ReadCause::GitFile(path) => format!(".git/{} changed", slashes(path)),
        ReadCause::File(path) => format!("{} changed", slashes(path)),
        ReadCause::Saved(path) => format!("{} was saved", slashes(path)),
        ReadCause::Files(FilesChange::Created) => String::from("files were created"),
        ReadCause::Files(FilesChange::Deleted) => String::from("files were deleted"),
        ReadCause::Files(FilesChange::Renamed) => String::from("files were renamed"),
        ReadCause::Focus => String::from("the window got the focus"),
        ReadCause::Settings => String::from("the settings changed"),
        ReadCause::Refresh => String::from("Refresh was asked for"),
        ReadCause::OperationEnded => String::from("an operation ended"),
        ReadCause::RepositoryChosen => String::from("the repository was chosen"),
        ReadCause::BranchesShown => String::from("the branches shown changed"),
        ReadCause::Search => String::from("the search changed"),
        ReadCause::Filter => String::from("the filter changed"),
        ReadCause::NextPage => String::from("more commits were asked for"),
    }
}

/// A path with forward slashes, as the log and the Activity tab write paths on every system.
fn slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
