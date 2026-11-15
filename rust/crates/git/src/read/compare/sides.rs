//! Everything the comparison page shows of two sides but the lines of the files.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::thread::{self, ScopedJoinHandle};

use model::CommitId;
use model::compare::{COMMIT_LIMIT, CompareMode, CompareView, MergeOutlook, Side};

use super::parse::{
    Counts, parse_cherry_marks, parse_compared_commits, parse_counts, parse_merge_base, parse_merge_tree,
};
use super::{CompareRequest, NOT_COMMITS};
use crate::read::changed_files::parse_name_status;
use crate::read::existing::is_there;
use crate::{Git, GitError, command_line};

/// The two sides as git is given them, the working tree standing for `HEAD`, and the path the comparison is narrowed
/// to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revisions {
    /// The first side: `<ours>`.
    pub first: String,
    /// The second side: `<theirs>`.
    pub second: String,
    /// The file or folder it is narrowed to.
    pub path: Option<String>,
}

impl Revisions {
    /// The revisions of a comparison.
    #[must_use]
    pub fn of(request: &CompareRequest) -> Revisions {
        Revisions {
            first: revision(&request.first),
            second: revision(&request.second),
            path: request.path.clone(),
        }
    }

    /// Some words of a command, followed by `--` and the path as a literal pathspec when the comparison is narrowed.
    fn narrowed(&self, words: &[&str]) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = words.iter().map(OsString::from).collect();
        arguments.extend(pathspec(self.path.as_deref()));
        arguments
    }

    /// `<first>...<second>`, or the other way round.
    fn symmetric(&self, order: Order) -> String {
        match order {
            Order::FirstThenSecond => format!("{}...{}", self.first, self.second),
            Order::SecondThenFirst => format!("{}...{}", self.second, self.first),
        }
    }
}

/// Which side is written first in a range.
#[derive(Debug, Clone, Copy)]
enum Order {
    FirstThenSecond,
    SecondThenFirst,
}

/// `git merge-base <ours> <theirs>`: where they diverged. It takes no path.
#[must_use]
pub fn merge_base_arguments(revisions: &Revisions) -> Vec<OsString> {
    ["merge-base", revisions.first.as_str(), revisions.second.as_str()]
        .into_iter()
        .map(OsString::from)
        .collect()
}

/// `git rev-list --left-right --count <theirs>...<ours> --`: how many commits each has that the other has not, the
/// whole sides also when the comparison is narrowed, as its header says how far apart the two are.
#[must_use]
pub fn counts_arguments(revisions: &Revisions) -> Vec<OsString> {
    let range = revisions.symmetric(Order::SecondThenFirst);
    let mut arguments: Vec<OsString> = ["rev-list", "--left-right", "--count", &range]
        .into_iter()
        .map(OsString::from)
        .collect();
    arguments.extend(pathspec(None));
    arguments
}

/// `git log --left-right --max-count=2000` of `<ours>...<theirs>`, each record its side mark, hash, author's name and
/// date, and subject, separated by the unit separator and ended by the record separator.
#[must_use]
pub fn commits_arguments(revisions: &Revisions) -> Vec<OsString> {
    let count = format!("--max-count={COMMIT_LIMIT}");
    let range = revisions.symmetric(Order::FirstThenSecond);
    revisions.narrowed(&[
        "log",
        "--left-right",
        &count,
        "--format=%m%x1f%H%x1f%an%x1f%at%x1f%s%x1e",
        &range,
    ])
}

/// `git rev-list --cherry-mark <ours>...<theirs>`: a commit marked `=` has its change on the other side too.
#[must_use]
pub fn cherry_arguments(revisions: &Revisions) -> Vec<OsString> {
    let range = revisions.symmetric(Order::FirstThenSecond);
    revisions.narrowed(&["rev-list", "--cherry-mark", &range])
}

/// `git merge-tree --write-tree --no-messages --name-only -z <ours> <theirs>`: the files a merge of the two would leave
/// in conflict. It takes no path.
#[must_use]
pub fn merge_tree_arguments(revisions: &Revisions) -> Vec<OsString> {
    [
        "merge-tree",
        "--write-tree",
        "--no-messages",
        "--name-only",
        "-z",
        revisions.first.as_str(),
        revisions.second.as_str(),
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// `git diff --name-status -z -M` of the files that differ: from `<since>` to the second side, `<since>` alone when the
/// second side is the working tree, and `-R <second>` when the first is, but in **Since they diverged**, where the files
/// are those the second side changed since the merge base.
#[must_use]
pub fn compare_files_arguments(request: &CompareRequest, merge_base: Option<&CommitId>) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = ["diff", "--name-status", "-z", "-M"]
        .into_iter()
        .map(OsString::from)
        .collect();
    arguments.extend(range(request, merge_base).into_iter().map(OsString::from));
    arguments.extend(pathspec(request.path.as_deref()));
    arguments
}

/// Everything the comparison page shows of two sides but the lines of the files: where they diverged, how many commits
/// each has that the other has not, the files that differ as the mode says, the commits only in each (newest first, at
/// most 2,000, those whose change is also on the other side marked when there are 2,000 or fewer), and what merging them
/// would do. A comparison narrowed to a path reads only the files and the commits that touch it: how far apart the two
/// sides are, and what merging them would do, are of the whole sides.
///
/// Feature: compare.read-sides
///
/// # Errors
///
/// Refuses two sides that are both the working tree, or that are not commits of the repository, with **These are not
/// commits of this repository.**; fails when git cannot be started, fails, or prints what this does not understand.
pub fn read_sides(git: &Git, request: &CompareRequest) -> Result<CompareView, GitError> {
    if request.first == Side::WorkingTree && request.second == Side::WorkingTree {
        return Err(not_commits());
    }
    let revisions = Revisions::of(request);
    let merge_base = read_merge_base(git, request, &revisions)?;
    let (counts, mut commits, files) = thread::scope(|scope| {
        let counts = scope.spawn(|| parse_counts(&git.run_read(&counts_arguments(&revisions))?));
        let commits = scope.spawn(|| parse_compared_commits(&git.run_read(&commits_arguments(&revisions))?));
        let files = git
            .run_read(&compare_files_arguments(request, merge_base.as_ref()))
            .and_then(|output| parse_name_status(&output));
        Ok::<_, GitError>((joined(counts)?, joined(commits)?, files?))
    })?;
    let (marked, merge) = read_after_counts(git, request, &revisions, counts)?;
    for commit in &mut commits {
        commit.is_also_in_other_side = marked.contains(&commit.id);
    }
    Ok(CompareView {
        merge_base,
        only_in_first: counts.only_in_first,
        only_in_second: counts.only_in_second,
        files,
        commits,
        merge,
    })
}

/// The revisions git is given for the files that differ, as [`compare_files_arguments`] says.
pub(super) fn range(request: &CompareRequest, merge_base: Option<&CommitId>) -> Vec<String> {
    match (&request.first, &request.second) {
        (Side::WorkingTree, Side::Commit(second)) => match (request.mode, merge_base) {
            (CompareMode::SinceDiverged, Some(base)) => vec![base.to_string(), second.to_string()],
            _ => vec![String::from("-R"), second.to_string()],
        },
        (Side::WorkingTree, Side::WorkingTree) => Vec::new(),
        (Side::Commit(first), second) => {
            let since = match (request.mode, merge_base) {
                (CompareMode::SinceDiverged, Some(base)) => base.to_string(),
                (CompareMode::SinceDiverged | CompareMode::Everything, _) => first.to_string(),
            };
            match second {
                Side::Commit(second) => vec![since, second.to_string()],
                Side::WorkingTree => vec![since],
            }
        }
    }
}

/// `--`, and the path as a literal pathspec when there is one, so that `pages/[id].tsx` is that file only.
pub(super) fn pathspec(path: Option<&str>) -> Vec<OsString> {
    let mut words = vec![OsString::from("--")];
    words.extend(path.map(|path| OsString::from(format!(":(literal){path}"))));
    words
}

/// The refusal of sides that are not commits of the repository.
pub(super) fn not_commits() -> GitError {
    GitError::Refused {
        reason: NOT_COMMITS.to_owned(),
    }
}

/// The working tree stands for `HEAD`.
fn revision(side: &Side) -> String {
    match side {
        Side::Commit(commit) => commit.to_string(),
        Side::WorkingTree => String::from("HEAD"),
    }
}

/// Where the two diverged; none when their histories never meet. When git fails, a side that is not a commit of the
/// repository is the reason, and the comparison is refused.
fn read_merge_base(git: &Git, request: &CompareRequest, revisions: &Revisions) -> Result<Option<CommitId>, GitError> {
    let arguments = merge_base_arguments(revisions);
    let output = git.read_output(&arguments)?;
    match output.code {
        Some(0) => return parse_merge_base(&output.stdout),
        Some(1) => return Ok(None),
        _ => {}
    }
    for side in [&request.first, &request.second] {
        if let Side::Commit(commit) = side
            && !is_there(git, &format!("{commit}^{{commit}}"))?
        {
            return Err(not_commits());
        }
    }
    Err(GitError::Failed {
        command: command_line(&arguments),
        code: output.code,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// The commits whose change is also on the other side, when there are 2,000 commits or fewer between them, and what
/// merging would do, when neither side is the working tree and each has commits the other has not.
fn read_after_counts(
    git: &Git,
    request: &CompareRequest,
    revisions: &Revisions,
    counts: Counts,
) -> Result<(BTreeSet<CommitId>, MergeOutlook), GitError> {
    let asks_cherries = counts.total() > 0 && counts.total() <= COMMIT_LIMIT;
    let asks_merge = request.first != Side::WorkingTree
        && request.second != Side::WorkingTree
        && counts.only_in_first > 0
        && counts.only_in_second > 0;
    thread::scope(|scope| {
        let marked =
            asks_cherries.then(|| scope.spawn(|| parse_cherry_marks(&git.run_read(&cherry_arguments(revisions))?)));
        let merge = if asks_merge {
            parse_merge_tree(&git.read_output(&merge_tree_arguments(revisions))?)
        } else {
            MergeOutlook::NotAsked
        };
        let marked = match marked {
            Some(reader) => joined(reader)?,
            None => BTreeSet::new(),
        };
        Ok((marked, merge))
    })
}

/// What a read on a thread of its own returned. A read that panicked panics here too, as it would have without the
/// thread.
fn joined<Value>(reader: ScopedJoinHandle<'_, Value>) -> Value {
    reader
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}
