//! The worktrees of a repository and how they stand, and the links to folders outside one that git would follow when
//! it removes it.

mod folder_links;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::thread;

use model::CommitId;
use model::state::Worktree;

use crate::read::fields::{arguments, optional_commit_id, unexpected};
use crate::read::repo::{joined, parse_status, worktree_status_arguments};
use crate::{Git, GitError};

pub use folder_links::{FolderLinksRequest, folder_links_arguments, parse_folder_entries, read_folder_links};

/// What is being read, for the errors.
const WHAT: &str = "the worktrees";

/// Which worktrees: those of the repository of the folder the views show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeListRequest {
    /// The top folder of the worktree the views show, as git names it (`git rev-parse --show-toplevel`).
    pub folder: PathBuf,
    /// How many files are not committed in that worktree, as the Changes pane counted them; none when not read yet.
    pub uncommitted_here: Option<usize>,
}

/// The worktrees of the repository, the main one first: the folder of each, the commit it has checked out, its branch
/// or none when detached, whether it is the one the views show and whether its folder is missing. Each other worktree
/// that is not missing has the number of its files not committed, all read at once and without a lock; one that git
/// cannot read has none, and the others are counted all the same.
///
/// Feature: worktree.read-list
///
/// # Errors
///
/// When git cannot be started or fails, or prints a worktree this does not understand.
pub fn read_list(git: &Git, request: &WorktreeListRequest) -> Result<Vec<Worktree>, GitError> {
    let listed = parse_worktrees(&git.in_directory(&request.folder).run_read(&worktrees_arguments())?)?;
    let counts: Vec<Option<usize>> = thread::scope(|scope| {
        let readers: Vec<_> = listed
            .iter()
            .map(|worktree| {
                let is_other = !worktree.is_missing && worktree.folder != request.folder;
                is_other.then(|| scope.spawn(|| count_uncommitted(git, &worktree.folder)))
            })
            .collect();
        readers.into_iter().map(|reader| reader.and_then(joined)).collect()
    });
    Ok(listed
        .into_iter()
        .zip(counts)
        .map(|(worktree, uncommitted)| {
            let is_current = worktree.folder == request.folder;
            Worktree {
                is_current,
                uncommitted: if is_current {
                    request.uncommitted_here
                } else {
                    uncommitted
                },
                ..worktree
            }
        })
        .collect())
}

/// `git worktree list --porcelain -z`: one attribute per field, and an empty field after each worktree.
#[must_use]
pub fn worktrees_arguments() -> Vec<OsString> {
    arguments(&["worktree", "list", "--porcelain", "-z"])
}

/// The worktrees that `git worktree list --porcelain -z` printed, the main one first, without a bare repository. None
/// is marked as the one the views show, and none has its count of files not committed: [`read_list`] adds both.
///
/// # Errors
///
/// When a worktree has no folder, or its commit is not a hash.
pub fn parse_worktrees(output: &[u8]) -> Result<Vec<Worktree>, GitError> {
    let text = String::from_utf8_lossy(output);
    let entries = text
        .split("\0\0")
        .filter(|record| !record.is_empty())
        .map(entry)
        .collect::<Result<Vec<Entry>, GitError>>()?;
    Ok(entries
        .into_iter()
        .enumerate()
        .filter(|(_, entry)| !entry.is_bare)
        .map(|(position, entry)| Worktree {
            folder: entry.folder,
            commit: entry.commit,
            branch: entry.branch,
            is_main: position == 0,
            is_current: false,
            is_missing: entry.is_missing,
            uncommitted: None,
        })
        .collect())
}

/// How many files are not committed in another worktree: staged, not staged and untracked alike.
fn count_uncommitted(git: &Git, folder: &Path) -> Option<usize> {
    // A worktree that git cannot read, such as one whose folder was replaced by something else, counts nothing rather
    // than failing the read of the one shown: the list says it is there, without a count.
    let output = git.in_directory(folder).run_read(&worktree_status_arguments()).ok()?;
    parse_status(&output).ok().map(|working_tree| working_tree.file_count())
}

/// The attributes of one worktree, as they are read.
struct Entry {
    folder: PathBuf,
    commit: Option<CommitId>,
    branch: Option<String>,
    is_bare: bool,
    is_missing: bool,
}

/// One worktree, from its attributes: one per field, a name and a value.
fn entry(record: &str) -> Result<Entry, GitError> {
    let mut folder = None;
    let mut entry = Entry {
        folder: PathBuf::new(),
        commit: None,
        branch: None,
        is_bare: false,
        is_missing: false,
    };
    for field in record.split('\0') {
        let (name, value) = field.split_once(' ').unwrap_or((field, ""));
        match name {
            "worktree" => folder = Some(PathBuf::from(value)),
            "HEAD" => entry.commit = optional_commit_id(WHAT, value)?,
            "branch" => entry.branch = Some(value.to_owned()),
            "bare" => entry.is_bare = true,
            "prunable" => entry.is_missing = true,
            // `detached`, `locked` and what a later git adds say nothing the list keeps.
            _ => {}
        }
    }
    entry.folder = folder.ok_or_else(|| unexpected(WHAT, record))?;
    Ok(entry)
}
