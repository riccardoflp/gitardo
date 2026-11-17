//! Whether something in the own git folder of a worktree changed since it was looked at before: its `HEAD`, its index,
//! the files an operation leaves there while it stops half way, the reflog of its `HEAD` (M15). A read of the state
//! runs `git status` in another worktree only when this says so (`repo.read-state`); a read of a worktree in the
//! background may ask it too. It reads the times and sizes of a few files, and runs no git.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The folders of a git folder whose files are looked at too: what a rebase, a cherry-pick or a revert of a series
/// leaves while it stops half way.
const OPERATION_FOLDERS: [&str; 3] = ["rebase-merge", "rebase-apply", "sequencer"];

/// The socket the daemon of `core.fsmonitor` listens on beside its folder, which changes while nothing else does.
const FSMONITOR_SOCKET: &str = "fsmonitor--daemon.ipc";

/// What the files of a worktree's own git folder were when they were looked at: the size and the time of each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitFolderStamp {
    /// Each file, by its path from the git folder with `/`.
    files: BTreeMap<String, FileStamp>,
}

/// The size of a file and when it was last written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    /// Its size, in bytes.
    length: u64,
    /// When it was last written, when the system says.
    modified: Option<SystemTime>,
}

/// What [`git_folder_changed`] found: whether something changed, and the stamp of now, for the next time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFolderCheck {
    /// Whether something in the git folder changed since the stamp given, or there was none, or the git folder could
    /// not be found.
    pub is_changed: bool,
    /// The stamp of the git folder now.
    pub stamp: GitFolderStamp,
}

/// Whether something in the own git folder of the worktree in `worktree` changed since `before` was taken: the git
/// folder `.git` names, `.git/worktrees/<name>/` for a linked worktree, `.git` itself for the main one. Its files are
/// looked at, the reflog of its `HEAD`, and those of a rebase, a cherry-pick or a revert stopped half way; not the
/// locks, which are always followed by the file they lock, nor what the daemon of `core.fsmonitor` writes each time
/// `git status` runs there. Without a stamp before, or when the git folder cannot be found, it changed.
#[must_use]
pub fn git_folder_changed(worktree: &Path, before: Option<&GitFolderStamp>) -> GitFolderCheck {
    let stamp = own_git_folder(worktree).map_or_else(GitFolderStamp::default, |folder| stamp_of(&folder));
    let is_changed = stamp.files.is_empty() || before != Some(&stamp);
    GitFolderCheck { is_changed, stamp }
}

/// The own git folder of a worktree: `.git` when it is a folder, else the folder its `gitdir:` line names.
fn own_git_folder(worktree: &Path) -> Option<PathBuf> {
    let dot_git = worktree.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let named = text.lines().find_map(|line| line.strip_prefix("gitdir:"))?.trim();
    Some(worktree.join(named))
}

/// The stamp of a git folder: its files, `logs/HEAD`, and the files of the folders of an operation.
fn stamp_of(folder: &Path) -> GitFolderStamp {
    let mut files = BTreeMap::new();
    add_files(folder, "", &mut files);
    for operation in OPERATION_FOLDERS {
        add_files(&folder.join(operation), operation, &mut files);
    }
    if let Some(stamp) = file_stamp(&folder.join("logs").join("HEAD")) {
        let _new = files.insert(String::from("logs/HEAD"), stamp);
    }
    GitFolderStamp { files }
}

/// Adds the files directly in a folder, by their path from the git folder, but the locks and the socket of the daemon
/// of `core.fsmonitor`. A folder that is not there adds nothing.
fn add_files(folder: &Path, prefix: &str, files: &mut BTreeMap<String, FileStamp>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_lock = Path::new(&name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lock"));
        let is_skipped = is_lock || name == FSMONITOR_SOCKET;
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if is_skipped || !metadata.is_file() {
            continue;
        }
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let _new = files.insert(
            path,
            FileStamp {
                length: metadata.len(),
                modified: metadata.modified().ok(),
            },
        );
    }
}

/// The stamp of one file, none when it is not there.
fn file_stamp(path: &Path) -> Option<FileStamp> {
    let metadata = fs::metadata(path).ok()?;
    Some(FileStamp {
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::git_folder_changed;

    #[test]
    fn a_worktree_looked_at_twice_has_not_changed_and_git_add_there_changes_it() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
        let folder = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &folder.to_string_lossy()]);
        let first = git_folder_changed(&folder, None);
        assert!(first.is_changed, "nothing was looked at before");
        let _status = crate::Git::new(&folder)
            .run_read(&["status", "--porcelain=v2"])
            .unwrap();
        let second = git_folder_changed(&folder, Some(&first.stamp));
        assert!(!second.is_changed, "{:?} then {:?}", first.stamp, second.stamp);
        std::fs::write(folder.join("b.txt"), "b\n").unwrap();
        let _added = crate::Git::new(&folder).run_read(&["add", "b.txt"]).unwrap();
        assert!(git_folder_changed(&folder, Some(&second.stamp)).is_changed);
    }

    #[test]
    fn a_folder_that_is_not_a_worktree_has_always_changed() {
        let folder = std::env::temp_dir().join("gitardo-no-worktree-here");
        let check = git_folder_changed(&folder, None);
        assert!(check.is_changed);
        assert!(git_folder_changed(&folder, Some(&check.stamp)).is_changed);
    }
}
