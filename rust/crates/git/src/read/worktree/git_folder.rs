//! Whether something in the own git folder of a worktree changed since it was looked at before: its `HEAD`, its index,
//! the files an operation leaves there while it stops half way, the reflog of its `HEAD` (M15); and whether a branch
//! or a remote branch of its repository moved. A read of the state runs `git status` in another worktree only when its
//! own git folder changed (`repo.read-state`); a read of what a worktree is on in the background runs it also when a
//! branch moved, which may change how far it is from its upstream (`repo.worktree-summary`). And whether anything a
//! read of the repository shows changed, a tag, a stash or another worktree too, which is how a change no watcher said
//! is found (`repo.watch-git`, D93). It is the one place that says which files of a git folder count; it reads the
//! times and sizes of a few files and folders, and runs no git.

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

/// The folder of the common git folder whose times say that a tag was made or deleted, with the folders under it.
const TAGS_FOLDER: &str = "refs/tags";

/// The files of the common git folder that say which stashes there are: the reference of the newest, and the log
/// that lists them all.
const STASH_FILES: [&str; 2] = ["refs/stash", "logs/refs/stash"];

/// The folder of the common git folder that holds the own git folder of each linked worktree.
const WORKTREES_FOLDER: &str = "worktrees";

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
    /// What else of the common git folder a read of the repository shows: the folders of the tags, the files of the
    /// stashes, the folder of the worktrees and the files of the own git folder of each, by their path from it with
    /// `/`.
    others: BTreeMap<String, FileStamp>,
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
    /// Whether anything looked at differs from the stamp given, or there was none: the own git folder, the branches,
    /// the tags, the stashes or the worktrees of the repository. A folder that is not a worktree, looked at twice, did
    /// not change.
    pub is_anything_changed: bool,
    /// The stamp of the git folder now.
    pub stamp: GitFolderStamp,
}

/// Whether something in the own git folder of the worktree in `worktree` changed since `before` was taken: the git
/// folder `.git` names, `.git/worktrees/<name>/` for a linked worktree, `.git` itself for the main one. Its files are
/// looked at, the reflog of its `HEAD`, and those of a rebase, a cherry-pick or a revert stopped half way; not the
/// locks, which are always followed by the file they lock, nor what the daemon of `core.fsmonitor` writes each time
/// `git status` runs there. Without a stamp before, or when the git folder cannot be found, it changed. The branches
/// moved when `packed-refs` or a folder of `refs/heads` or `refs/remotes` of the common git folder changed. Anything
/// changed when one of those did, or a folder of `refs/tags`, `refs/stash` or its log, the folder `worktrees` or a
/// file of the own git folder of another worktree, its locks and the socket of its daemon left out again: it is what
/// finds a commit, a branch, a tag, a stash or a worktree made outside that no watcher said.
///
/// Feature: repo.watch-git
#[must_use]
pub fn git_folder_changed(worktree: &Path, before: Option<&GitFolderStamp>) -> GitFolderCheck {
    let stamp = own_git_folder(worktree).map_or_else(GitFolderStamp::default, |folder| stamp_of(&folder));
    let is_changed = stamp.files.is_empty() || before.is_none_or(|before| before.files != stamp.files);
    let have_branches_moved = before.is_none_or(|before| before.references != stamp.references);
    let is_anything_changed = before.is_none_or(|before| *before != stamp);
    GitFolderCheck {
        is_changed,
        have_branches_moved,
        is_anything_changed,
        stamp,
    }
}

/// The git folders of a worktree: its own, and the one its repository shares, which is the same for a main worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFolders {
    /// The git folder `.git` names: `.git` itself for the main worktree, `.git/worktrees/<name>` for a linked one.
    pub own: PathBuf,
    /// The git folder the worktrees of the repository share, where the references are.
    pub common: PathBuf,
}

/// The git folders of the worktree in `worktree`, found by reading `.git` and `commondir` and with no process of git,
/// each as the system writes it whole; none when the folder is not a worktree. They are what is watched to hear of a
/// change of the repository made outside (`repo.watch-git`).
#[must_use]
pub fn git_folders(worktree: &Path) -> Option<GitFolders> {
    let own = own_git_folder(worktree)?;
    let common = common_git_folder(&own);
    Some(GitFolders {
        own: fs::canonicalize(&own).ok()?,
        common: fs::canonicalize(&common).ok()?,
    })
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

/// The stamp of a git folder: its files, `logs/HEAD`, and the files of the folders of an operation; the references of
/// its common git folder; and the tags, the stashes and the worktrees there.
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
    GitFolderStamp {
        files,
        references,
        others: others_of(&common),
    }
}

/// What else of a common git folder is looked at: the folders of the tags, the files of the stashes, the folder of the
/// worktrees and the files directly in the own git folder of each, where its `HEAD` and its index are.
fn others_of(common: &Path) -> BTreeMap<String, FileStamp> {
    let mut others = BTreeMap::new();
    add_folders(&common.join(TAGS_FOLDER), TAGS_FOLDER, &mut others);
    for name in STASH_FILES {
        if let Some(stamp) = file_stamp(&common.join(name)) {
            let _new = others.insert(name.to_owned(), stamp);
        }
    }
    let worktrees = common.join(WORKTREES_FOLDER);
    let Some(stamp) = file_stamp(&worktrees) else {
        return others;
    };
    let _new = others.insert(WORKTREES_FOLDER.to_owned(), stamp);
    let Ok(entries) = fs::read_dir(&worktrees) else {
        return others;
    };
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            add_files(&entry.path(), &format!("{WORKTREES_FOLDER}/{name}"), &mut others);
        }
    }
    others
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
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{GitFolderStamp, git_folder_changed};

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
        let again = git_folder_changed(&folder, Some(&check.stamp));
        assert!(again.is_changed);
        assert!(!again.is_anything_changed, "nothing is there, as before");
    }

    /// A repository with a commit and a file changed, to stash, and what its git folder was like.
    fn looked_at() -> (testkit::Repository, GitFolderStamp) {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
        let stamp = git_folder_changed(repository.path(), None).stamp;
        (repository, stamp)
    }

    /// Whether anything changed in the git folder of a repository since a stamp, and the stamp of now.
    fn changed_since(repository: &testkit::Repository, stamp: &GitFolderStamp) -> (bool, GitFolderStamp) {
        let check = git_folder_changed(repository.path(), Some(stamp));
        (check.is_anything_changed, check.stamp)
    }

    #[test]
    fn repo_watch_git__a_commit_a_branch_made_and_deleted_a_tag_a_stash_and_a_worktree_each_change_the_folder() {
        let (mut repository, stamp) = looked_at();
        let (nothing, stamp) = changed_since(&repository, &stamp);
        assert!(!nothing, "nothing was written");
        let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
        let (committed, stamp) = changed_since(&repository, &stamp);
        assert!(committed, "a commit");
        let _made = repository.git(&["branch", "feature"]);
        let (branched, stamp) = changed_since(&repository, &stamp);
        assert!(branched, "a new branch");
        let _deleted = repository.git(&["branch", "-D", "feature"]);
        let (deleted, stamp) = changed_since(&repository, &stamp);
        assert!(deleted, "a branch deleted");
        let _tagged = repository.git(&["tag", "v1"]);
        let (tagged, stamp) = changed_since(&repository, &stamp);
        assert!(tagged, "a tag");
        std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
        let (_written, stamp) = changed_since(&repository, &stamp);
        let _stashed = repository.git(&["stash"]);
        let (stashed, stamp) = changed_since(&repository, &stamp);
        assert!(stashed, "a stash");
        let folder = repository.path().with_file_name(format!(
            "{}-other",
            repository.path().file_name().unwrap().to_string_lossy()
        ));
        let _added = repository.git(&["worktree", "add", "-b", "other", &folder.to_string_lossy()]);
        let (added, stamp) = changed_since(&repository, &stamp);
        assert!(added, "a worktree added");
        let (nothing, _stamp) = changed_since(&repository, &stamp);
        assert!(!nothing, "nothing was written since");
        let _removed = repository.git(&["worktree", "remove", "--force", &folder.to_string_lossy()]);
    }

    #[test]
    fn repo_watch_git__a_tag_a_stash_and_a_worktree_move_no_branch_and_leave_the_own_git_folder_of_another_worktree() {
        let (repository, _stamp) = looked_at();
        let folder = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &folder.to_string_lossy()]);
        let first = git_folder_changed(&folder, None);
        let _tagged = repository.git(&["tag", "v1"]);
        let check = git_folder_changed(&folder, Some(&first.stamp));
        assert!(check.is_anything_changed);
        assert!(!check.is_changed, "{:?} then {:?}", first.stamp, check.stamp);
        assert!(!check.have_branches_moved, "{:?} then {:?}", first.stamp, check.stamp);
    }

    #[test]
    fn repo_watch_git__git_status_in_the_worktree_and_in_another_one_changes_nothing() {
        let (repository, _stamp) = looked_at();
        let folder = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &folder.to_string_lossy()]);
        let stamp = git_folder_changed(repository.path(), None).stamp;
        for tree in [repository.path(), folder.as_path()] {
            let _status = crate::Git::new(tree).run_read(&["status", "--porcelain=v2"]).unwrap();
        }
        let check = git_folder_changed(repository.path(), Some(&stamp));
        assert!(!check.is_anything_changed, "{stamp:?} then {:?}", check.stamp);
    }

    #[test]
    fn repo_watch_git__a_lock_and_what_the_daemon_of_fsmonitor_writes_in_a_worktree_change_nothing() {
        let (repository, _stamp) = looked_at();
        let folder = repository.path().join("feature");
        let _added = repository.git(&["worktree", "add", "-b", "feature", &folder.to_string_lossy()]);
        let stamp = git_folder_changed(repository.path(), None).stamp;
        let own = repository.path().join(".git").join("worktrees").join("feature");
        std::fs::write(own.join("index.lock"), "").unwrap();
        std::fs::write(own.join("fsmonitor--daemon.ipc"), "").unwrap();
        std::fs::create_dir_all(own.join("fsmonitor--daemon").join("cookies")).unwrap();
        std::fs::write(own.join("fsmonitor--daemon").join("cookies").join("1-2"), "").unwrap();
        std::fs::write(repository.path().join(".git").join("index.lock"), "").unwrap();
        let check = git_folder_changed(repository.path(), Some(&stamp));
        assert!(!check.is_anything_changed, "{stamp:?} then {:?}", check.stamp);
    }

    #[test]
    fn repo_watch_git__it_starts_no_process_of_git() {
        // A folder that only looks like a git folder, which git would refuse: what is found there is found by reading.
        let folder = std::env::temp_dir().join(format!("gitardo-looks-like-git-{}", std::process::id()));
        let _gone = std::fs::remove_dir_all(&folder);
        let heads = folder.join(".git").join("refs").join("heads");
        std::fs::create_dir_all(&heads).unwrap();
        std::fs::write(folder.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let first = git_folder_changed(&folder, None);
        assert!(!git_folder_changed(&folder, Some(&first.stamp)).is_anything_changed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(heads.join("main"), "0123456789012345678901234567890123456789\n").unwrap();
        assert!(git_folder_changed(&folder, Some(&first.stamp)).is_anything_changed);
        std::fs::remove_dir_all(&folder).unwrap();
        // And nothing in the code of this file, before its tests, names what starts a program.
        let code = include_str!("git_folder.rs").split("#[cfg(test)]").next().unwrap();
        for starts in ["Command", "Git::", "run_read", "process::"] {
            assert!(!code.contains(starts), "{starts}");
        }
    }
}
