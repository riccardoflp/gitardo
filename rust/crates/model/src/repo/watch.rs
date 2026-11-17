//! Which changes ask for a read of the repository, after how long, and what each is said as (`repo.watch-changes`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::state::RepositoryState;

/// How long a change in the git folder, a file Gitardo saved or created, the focus or a setting waits before the read.
pub const SOON: Duration = Duration::from_millis(400);
/// How long a change in the working tree waits before the read: builds and installs write many files at once.
pub const LATER: Duration = Duration::from_millis(1_000);
/// How long after the last read the window getting the focus asks for another (M15).
pub const FOCUS_AFTER: Duration = Duration::from_secs(60);

/// The files of the common git folder whose change asks for a read, by their names at its top.
const GIT_FILES: [&str; 10] = [
    "HEAD",
    "FETCH_HEAD",
    "ORIG_HEAD",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REVERT_HEAD",
    "BISECT_LOG",
    "index",
    "config",
    "packed-refs",
];

/// The sections of the settings whose change asks for a read: what they change is read with the repository.
const READ_SETTINGS: [&str; 5] = ["conflicts", "pull", "worktrees", "branches", "graph"];

/// The folder the daemon of `core.fsmonitor` writes in each time `git status` runs, which never asks for a read.
const FSMONITOR_FOLDER: &str = "fsmonitor--daemon";

/// The socket the daemon of `core.fsmonitor` listens on, on Linux and macOS, beside that folder: it never asks for a
/// read either.
const FSMONITOR_SOCKET: &str = "fsmonitor--daemon.ipc";

/// The files that say what git ignores: a change to one is never ignored itself, and asks for a read.
const IGNORE_FILE: &str = ".gitignore";

/// What a change of a file of the working tree needs before it asks for a read (M15).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeChange {
    /// It asks for a read: a `.gitignore`, or the whole working tree when the system could not say what changed in it.
    Read,
    /// Git is asked first, with the other files of its burst, whether it ignores the file (`git check-ignore`): only a
    /// file it does not ignore asks for a read.
    AskGit,
    /// It asks for nothing: it is in the folder of another worktree nested in this one.
    Nothing,
}

/// Something that changed, as the window learns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A file or a folder of the common git folder (`git rev-parse --git-common-dir`), by its path from it.
    GitFolder(PathBuf),
    /// A file or a folder of the working tree, by its path from its top.
    WorkingTree(PathBuf),
    /// A file saved by Gitardo's own editor, by its path from the top of the working tree.
    Saved(PathBuf),
    /// Files created, deleted or renamed by Gitardo.
    Files(FilesChange),
    /// The window got the focus.
    Focus(FocusChange),
    /// A setting changed, by its key: `graph.pageSize`.
    Setting(String),
}

/// The window got the focus: how long after the last read, and whether a change came that asked for nothing while the
/// window was minimized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FocusChange {
    /// How long since the last read of the repository shown ended; none when it was never read.
    pub since_read: Option<Duration>,
    /// Whether a change came while the window was minimized, which asked for nothing then.
    pub is_change_missed: bool,
}

/// What Gitardo did to files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilesChange {
    /// Files were created.
    Created,
    /// Files were deleted.
    Deleted,
    /// Files were renamed.
    Renamed,
}

/// A read asked for: after how long, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadRequest {
    /// How long after the change the read starts.
    pub delay: Duration,
    /// Why it is read, as the Activity tab says it.
    pub cause: ReadCause,
}

/// Why the repository is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadCause {
    /// A file of the git folder changed, by its path from it: `.git/refs/heads/main changed`.
    GitFile(PathBuf),
    /// A file of the working tree changed, by its path from its top: `src/a.rs changed`.
    File(PathBuf),
    /// Gitardo's editor saved a file: `src/a.rs was saved`.
    Saved(PathBuf),
    /// Gitardo created, deleted or renamed files.
    Files(FilesChange),
    /// The window got the focus.
    Focus,
    /// The settings changed.
    Settings,
    /// **Refresh** was asked for.
    Refresh,
    /// An operation ended.
    OperationEnded,
    /// The repository shown was chosen.
    RepositoryChosen,
    /// The branches the timeline is narrowed to changed.
    BranchesShown,
    /// A search narrows the timeline, or no longer does.
    Search,
    /// A filter of the timeline changed.
    Filter,
    /// The next page of the timeline was asked for.
    NextPage,
}

impl ReadCause {
    /// Whether a read for it may keep the commits the timeline has, when the commit checked out, the references and the
    /// stashes are as they were (`repo.schedule-reads`): a change of files, the focus, the end of an operation.
    /// **Refresh**, the settings, another repository, the branches shown, a search, a filter and the next page read the
    /// commits again.
    #[must_use]
    pub fn may_keep_commits(&self) -> bool {
        matches!(
            self,
            ReadCause::GitFile(_)
                | ReadCause::File(_)
                | ReadCause::Saved(_)
                | ReadCause::Files(_)
                | ReadCause::Focus
                | ReadCause::OperationEnded
        )
    }

    /// Whether the bar along the top of the list of commits runs until a read asked for this is done: the commits of a
    /// filter, of the branches just chosen or of the next page are what the person waits for.
    #[must_use]
    pub fn shows_bar(&self) -> bool {
        matches!(self, ReadCause::BranchesShown | ReadCause::Filter | ReadCause::NextPage)
    }
}

/// Whether a change asks for a read, after how long and why: none for the files of the daemon of `core.fsmonitor`, for
/// locks, for the parts of the git folder that say nothing of what the window shows, and for what is in a `.git` of
/// the working tree. These rules keep a repository from being read for ever by the reads themselves.
///
/// Feature: repo.watch-changes
#[must_use]
pub fn watch_changes(change: &Change) -> Option<ReadRequest> {
    let (delay, cause) = match change {
        Change::GitFolder(path) => {
            let parts = parts(path);
            (is_watched(&parts) && is_git_file_read(&parts)).then_some((SOON, ReadCause::GitFile(path.clone())))?
        }
        Change::WorkingTree(path) => {
            let parts = parts(path);
            let is_in_git = parts.iter().any(|part| part == ".git");
            (is_watched(&parts) && !is_in_git).then_some((LATER, ReadCause::File(path.clone())))?
        }
        Change::Saved(path) => (SOON, ReadCause::Saved(path.clone())),
        Change::Files(files) => (SOON, ReadCause::Files(*files)),
        Change::Focus(focus) => {
            let is_due = focus.is_change_missed || focus.since_read.is_none_or(|since| since > FOCUS_AFTER);
            is_due.then_some((SOON, ReadCause::Focus))?
        }
        Change::Setting(key) => {
            let section = key.split('.').next().unwrap_or_default();
            READ_SETTINGS
                .contains(&section)
                .then_some((SOON, ReadCause::Settings))?
        }
    };
    Some(ReadRequest { delay, cause })
}

/// What a change of a file of the working tree, by its path from the top, needs before it asks for a read: nothing in
/// the folder of another worktree nested in this one (`nested`, as [`nested_worktrees`] gives them), a read at once
/// for a `.gitignore` or the whole working tree, and a question to git for the others. What changes in a nested
/// worktree never changes what a read of this one shows, and its own git folder asks for the read a commit there needs.
#[must_use]
pub fn sort_tree_change(path: &Path, nested: &[String]) -> TreeChange {
    let parts = parts(path);
    let is_nested = nested.iter().any(|folder| {
        let folder: Vec<&str> = folder.split('/').collect();
        parts.len() >= folder.len() && parts.iter().zip(&folder).all(|(part, name)| same_name(part, name))
    });
    if is_nested {
        return TreeChange::Nothing;
    }
    match parts.last() {
        None => TreeChange::Read,
        Some(name) if same_name(name, IGNORE_FILE) => TreeChange::Read,
        Some(_) => TreeChange::AskGit,
    }
}

/// The folders of the other worktrees inside the working tree a state was read in, from its top with `/`, such as
/// `.claude/worktrees/feature`.
#[must_use]
pub fn nested_worktrees(state: &RepositoryState) -> Vec<String> {
    state
        .worktrees
        .iter()
        .filter(|worktree| !worktree.is_current)
        .filter_map(|worktree| super::path_in_folder(&worktree.folder, &state.folder))
        .filter(|inside| !inside.is_empty())
        .collect()
}

/// Whether two names of a path are the same: the case of a letter does not count on Windows.
fn same_name(first: &str, second: &str) -> bool {
    if cfg!(windows) {
        first.eq_ignore_ascii_case(second)
    } else {
        first == second
    }
}

/// Whether a path may ask for a read at all: not a path with a part named `fsmonitor--daemon` or
/// `fsmonitor--daemon.ipc`, nor a lock.
pub(crate) fn is_watched(parts: &[String]) -> bool {
    let is_daemon = parts
        .iter()
        .any(|part| part == FSMONITOR_FOLDER || part == FSMONITOR_SOCKET);
    let is_lock = parts.last().is_some_and(|name| {
        Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lock"))
    });
    !is_daemon && !is_lock
}

/// Whether a path of the common git folder is one the window reads: the files of [`GIT_FILES`], anything under
/// `refs/` and `worktrees/`, `logs/refs/stash`, and the `HEAD` and references of the submodules under `modules/`.
fn is_git_file_read(parts: &[String]) -> bool {
    let names: Vec<&str> = parts.iter().map(String::as_str).collect();
    match names.as_slice() {
        [name] => GIT_FILES.contains(name),
        ["refs" | "worktrees", _, ..] | ["logs", "refs", "stash"] => true,
        ["modules", ..] => crate::submodule::is_reference_of_submodule(&names),
        _ => false,
    }
}

/// The names of the folders and the file of a path, with either slash, `.` left out.
pub(crate) fn parts(path: &Path) -> Vec<String> {
    path.to_string_lossy()
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        Change, FilesChange, FocusChange, ReadCause, ReadRequest, TreeChange, sort_tree_change, watch_changes,
    };

    fn git(path: &str) -> Option<ReadRequest> {
        watch_changes(&Change::GitFolder(PathBuf::from(path)))
    }

    #[test]
    fn repo_watch_changes__a_commit_in_a_terminal_asks_for_a_read_after_400_ms_said_by_the_file() {
        for path in ["HEAD", "refs/heads/main", "logs/refs/stash", "index", "packed-refs"] {
            let request = git(path).unwrap();
            assert_eq!(request.delay, Duration::from_millis(400), "{path}");
            assert_eq!(request.cause, ReadCause::GitFile(PathBuf::from(path)));
        }
    }

    #[test]
    fn repo_watch_changes__saving_a_file_from_another_editor_asks_for_a_read_after_1000_ms() {
        let request = watch_changes(&Change::WorkingTree(PathBuf::from("src/a.rs"))).unwrap();
        assert_eq!(request.delay, Duration::from_millis(1_000));
        assert_eq!(request.cause, ReadCause::File(PathBuf::from("src/a.rs")));
    }

    #[test]
    fn repo_watch_changes__a_cookie_of_the_fsmonitor_daemon_asks_for_nothing() {
        assert_eq!(git("worktrees/feature/fsmonitor--daemon/cookies/1234-5"), None);
        assert_eq!(git("fsmonitor--daemon/cookies/1"), None);
        let in_tree = Change::WorkingTree(PathBuf::from(".git/fsmonitor--daemon/cookies/1"));
        assert_eq!(watch_changes(&in_tree), None);
    }

    #[test]
    fn repo_watch_changes__the_socket_of_the_fsmonitor_daemon_in_a_worktree_asks_for_nothing() {
        assert_eq!(git("worktrees/feature/fsmonitor--daemon.ipc"), None);
        assert_eq!(git("fsmonitor--daemon.ipc"), None);
    }

    #[test]
    fn repo_watch_changes__a_lock_asks_for_nothing() {
        assert_eq!(git("index.lock"), None);
        assert_eq!(git("refs/heads/main.lock"), None);
        assert_eq!(
            watch_changes(&Change::WorkingTree(PathBuf::from("Cargo.lock.lock"))),
            None
        );
    }

    #[test]
    fn repo_watch_changes__what_the_reads_leave_in_the_git_folder_asks_for_nothing() {
        for path in [
            "objects/ab/cdef",
            "logs/HEAD",
            "COMMIT_EDITMSG",
            "hooks/pre-commit",
            "modules/lib/objects/x",
        ] {
            assert_eq!(git(path), None, "{path}");
        }
        assert!(git("worktrees/feature/HEAD").is_some());
        assert!(git("modules/lib/core/HEAD").is_some());
        assert!(git("modules/lib/refs/heads/main").is_some());
        let in_dot_git = Change::WorkingTree(PathBuf::from("vendor/lib/.git/HEAD"));
        assert_eq!(watch_changes(&in_dot_git), None);
    }

    #[test]
    fn repo_watch_changes__a_build_output_is_asked_of_git_and_a_gitignore_asks_for_a_read() {
        let nested = [String::from(".claude/worktrees/feature")];
        let sort = |path: &str| sort_tree_change(&PathBuf::from(path), &nested);
        assert_eq!(sort("target/debug/app.exe"), TreeChange::AskGit);
        assert_eq!(sort("src/main.rs"), TreeChange::AskGit);
        assert_eq!(sort(".gitignore"), TreeChange::Read);
        assert_eq!(sort("rust/.gitignore"), TreeChange::Read);
        assert_eq!(sort("."), TreeChange::Read, "the whole working tree");
    }

    #[test]
    fn repo_watch_changes__a_file_of_a_nested_worktree_asks_for_nothing_and_its_git_folder_asks_for_a_read() {
        let nested = [String::from(".claude/worktrees/feature")];
        let sort = |path: &str| sort_tree_change(&PathBuf::from(path), &nested);
        assert_eq!(sort(".claude/worktrees/feature/src/a.rs"), TreeChange::Nothing);
        assert_eq!(sort(".claude/worktrees/feature/.gitignore"), TreeChange::Nothing);
        assert_eq!(sort(".claude/worktrees/feature-two/a.rs"), TreeChange::AskGit);
        assert_eq!(sort(".claude/settings.json"), TreeChange::AskGit);
        let commit = git("worktrees/feature/HEAD").unwrap();
        assert_eq!(commit.delay, Duration::from_millis(400));
    }

    #[test]
    fn repo_watch_changes__the_focus_asks_for_a_read_only_a_minute_after_the_last_or_after_a_change_missed() {
        let focus = |seconds: u64, is_change_missed: bool| {
            watch_changes(&Change::Focus(FocusChange {
                since_read: Some(Duration::from_secs(seconds)),
                is_change_missed,
            }))
        };
        assert_eq!(focus(10, false), None, "ten seconds after a read");
        assert_eq!(focus(120, false).map(|request| request.cause), Some(ReadCause::Focus));
        assert_eq!(
            focus(10, true).map(|request| request.delay),
            Some(Duration::from_millis(400))
        );
    }

    #[test]
    fn repo_watch_changes__gitardo_files_focus_and_settings_ask_after_400_ms() {
        let soon = Duration::from_millis(400);
        let saved = watch_changes(&Change::Saved(PathBuf::from("a.rs"))).unwrap();
        assert_eq!(
            (saved.delay, saved.cause),
            (soon, ReadCause::Saved(PathBuf::from("a.rs")))
        );
        let files = watch_changes(&Change::Files(FilesChange::Renamed)).unwrap();
        assert_eq!(files.cause, ReadCause::Files(FilesChange::Renamed));
        let focus = Change::Focus(FocusChange::default());
        assert_eq!(watch_changes(&focus).map(|request| request.delay), Some(soon));
        let graph = watch_changes(&Change::Setting(String::from("graph.pageSize"))).unwrap();
        assert_eq!(graph.cause, ReadCause::Settings);
        assert_eq!(watch_changes(&Change::Setting(String::from("theme"))), None);
    }
}
