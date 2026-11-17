//! Whether something in the own git folder of a worktree changed since it was looked at before: its `HEAD`, its index,
//! the files an operation leaves there while it stops half way, the reflog of its `HEAD` (M15); and whether a branch
//! or a remote branch of its repository moved. A read of the state runs `git status` in another worktree only when its
//! own git folder changed (`repo.read-state`); a read of what a worktree is on in the background runs it also when a
//! branch moved, which may change how far it is from its upstream (`repo.worktree-summary`). It is the one place that
//! says which files of a git folder count; it reads the times and sizes of a few files and folders, and runs no git.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The folders of a git folder whose files are looked at too: what a rebase, a cherry-pick or a revert of a series
/// leaves while it stops half way.
const OPERATION_FOLDERS: [&str; 3] = ["rebase-merge", "rebase-apply", "sequencer"];

/// The folders of the common git folder whose times say that a branch or a remote branch moved, with the folders under
/// them, since git writes a reference by renaming a file into its folder.
const REFERENCE_FOLDERS: [&str; 2] = ["refs/heads", "refs/remotes"];

/// The socket the daemon of `core.fsmonitor` listens on beside its folder, which changes while nothing else does.
const FSMONITOR_SOCKET: &str = "fsmonitor--daemon.ipc";

/// What the files of a worktree's own git folder, and the references of its repository, were when they were looked at:
/// the size and the time of each.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitFolderStamp {
    /// Each file of the own git folder, by its path from that folder with `/`.
    files: BTreeMap<String, FileStamp>,
    /// `packed-refs` and the folders of [`REFERENCE_FOLDERS`] of the common git folder, by their path from it with `/`.
    references: BTreeMap<String, FileStamp>,
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
    /// Whether something in the own git folder changed since the stamp given, or there was none, or the git folder
    /// could not be found.
    pub is_changed: bool,
    /// Whether a branch or a remote branch of the repository moved since the stamp given, or there was none.
    pub have_branches_moved: bool,
    /// The stamp of the git folder now.
    pub stamp: GitFolderStamp,
}

/// Whether something in the own git folder of the worktree in `worktree` changed since `before` was taken: the git
/// folder `.git` names, `.git/worktrees/<name>/` for a linked worktree, `.git` itself for the main one. Its files are
/// looked at, the reflog of its `HEAD`, and those of a rebase, a cherry-pick or a revert stopped half way; not the
/// locks, which are always followed by the file they lock, nor what the daemon of `core.fsmonitor` writes each time
/// `git status` runs there. Without a stamp before, or when the git folder cannot be found, it changed. The branches
/// moved when `packed-refs` or a folder of `refs/heads` or `refs/remotes` of the common git folder changed.
#[must_use]
pub fn git_folder_changed(worktree: &Path, before: Option<&GitFolderStamp>) -> GitFolderCheck {
    let stamp = own_git_folder(worktree).map_or_else(GitFolderStamp::default, |folder| stamp_of(&folder));
    let is_changed = stamp.files.is_empty() || before.is_none_or(|before| before.files != stamp.files);
    let have_branches_moved = before.is_none_or(|before| before.references != stamp.references);
    GitFolderCheck {
        is_changed,
        have_branches_moved,
        stamp,
    }
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

/// The common git folder of a worktree whose own git folder is this one: the folder its `commondir` names, else itself.
fn common_git_folder(own: &Path) -> PathBuf {
    fs::read_to_string(own.join("commondir"))
        .ok()
        .map_or_else(|| own.to_path_buf(), |text| own.join(text.trim()))
}

/// The stamp of a git folder: its files, `logs/HEAD`, and the files of the folders of an operation; and the references
/// of its common git folder.
fn stamp_of(folder: &Path) -> GitFolderStamp {
    let mut files = BTreeMap::new();
    add_files(folder, "", &mut files);
    for operation in OPERATION_FOLDERS {
        add_files(&folder.join(operation), operation, &mut files);
    }
    if let Some(stamp) = file_stamp(&folder.join("logs").join("HEAD")) {
        let _new = files.insert(String::from("logs/HEAD"), stamp);
    }
    let common = common_git_folder(folder);
    let mut references = BTreeMap::new();
    if let Some(stamp) = file_stamp(&common.join("packed-refs")) {
        let _new = references.insert(String::from("packed-refs"), stamp);
    }
    for name in REFERENCE_FOLDERS {
        add_folders(&common.join(name), name, &mut references);
    }
    GitFolderStamp { files, references }
}

/// Adds a folder and every folder under it, by their path from the git folder; a folder that is not there adds
/// nothing.
fn add_folders(folder: &Path, path: &str, folders: &mut BTreeMap<String, FileStamp>) {
    let Some(stamp) = file_stamp(folder) else {
        return;
    };
    let _new = folders.insert(path.to_owned(), stamp);
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            add_folders(&entry.path(), &format!("{path}/{name}"), folders);
        }
    }
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
    fn a_commit_in_another_worktree_moves_the_branches_and_leaves_the_own_git_folder() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
        let folder = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &folder.to_string_lossy()]);
        let first = git_folder_changed(&folder, None);
        assert!(first.have_branches_moved, "nothing was looked at before");
        let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
        let check = git_folder_changed(&folder, Some(&first.stamp));
        assert!(!check.is_changed, "{:?} then {:?}", first.stamp, check.stamp);
        assert!(check.have_branches_moved, "{:?} then {:?}", first.stamp, check.stamp);
    }

    #[test]
    fn a_folder_that_is_not_a_worktree_has_always_changed() {
        let folder = std::env::temp_dir().join("gitardo-no-worktree-here");
        let check = git_folder_changed(&folder, None);
        assert!(check.is_changed);
        assert!(git_folder_changed(&folder, Some(&check.stamp)).is_changed);
    }
}
