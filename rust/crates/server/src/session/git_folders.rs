//! The git folders of the repositories opened, watched by the engine itself (`repo.watch-git`, D93, version 1.15 of
//! the protocol). The extension hears of a change made outside through VS Code's watcher of the workspace, which starts
//! late and drops changes while many files are written; so the engine watches the common git folder of each repository
//! opened, and the worktree's own when it is elsewhere, those folders alone and never the working tree, and says what
//! it sees in `repository/gitFolderChanged`. And since a watcher can lose changes too, every two seconds it looks again
//! at the git folder of each repository a view shows (`repository/shown`), with [`git_folder_changed`] and no process
//! of git, and says what differs with nothing said of it. A repository opened that no view shows is watched and not
//! looked at again: it is compared once when the extension says it is shown.
//!
//! What a plan of the engine writes is not said: the changes seen while an operation runs wait for its end, and are
//! dropped when a plan ran, whose own `repository/state` follows; those the watcher still says a moment after are
//! dropped too.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use git::read::worktree::{GitFolderStamp, git_folder_changed, git_folders};
use model::repo::{Change, watch_changes};
use protocol::git_folders::{GIT_FOLDER, GitFolderChangedParams, ShownParams, ShownResult, methods};
use protocol::rpc::{ResponseError, notification_message};
use serde_json::Value;
use system::{WatchDeliver, WatchEvent, Watcher};

use super::{Session, json, locked};

/// How often the git folder of a repository shown is looked at again: a change its watcher lost is said within this
/// (Q1792).
const LOOK_INTERVAL: Duration = Duration::from_secs(2);

/// How long after a plan ran what the watcher says of its repository is dropped: the system may say what the plan
/// wrote a moment after its last step ended.
const QUIET_AFTER_PLAN: Duration = Duration::from_millis(300);

/// The variable of the environment that, set to `off`, keeps the engine from watching: the git folders are then only
/// looked at again, as on a disk whose changes the system does not say. The tests of the engine set it.
const WATCH_VARIABLE: &str = "GITARDO_ENGINE_WATCH";

/// The git folders of the repositories opened.
#[derive(Default)]
pub struct GitFoldersWatched {
    /// What is watched and looked at.
    watched: Mutex<Watched>,
}

impl fmt::Debug for GitFoldersWatched {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let watched = locked(&self.watched);
        formatter
            .debug_struct("GitFoldersWatched")
            .field("repositories", &watched.entries.len())
            .field("shown", &watched.shown)
            .finish_non_exhaustive()
    }
}

/// What is watched and looked at.
#[derive(Default)]
struct Watched {
    /// Each repository opened, by the name the extension gave it.
    entries: HashMap<String, Entry>,
    /// The watcher of each git folder watched; the worktrees of a repository share the one of its common git folder.
    watchers: HashMap<PathBuf, Watcher>,
    /// The repositories the extension said a view shows.
    shown: Vec<String>,
    /// Where the watchers say what they saw; none until the thread that takes it is started.
    told: Option<Sender<Told>>,
}

/// A repository opened.
struct Entry {
    /// The top of its working tree.
    folder: PathBuf,
    /// Its git folders watched: the common one, and its own when that is not inside it.
    git_folders: Vec<PathBuf>,
    /// What its git folder was when it was last looked at, or a change of it was last said.
    stamp: GitFolderStamp,
    /// Whether its watcher said anything since it was last looked at, be it a file that asks for no read: what differs
    /// then is what the watcher saw, and what asked for a read of it was said.
    is_heard: bool,
    /// Whether an operation of the engine runs in it.
    is_operating: bool,
    /// The paths seen while the operation runs.
    held: Vec<String>,
    /// Until when what the watcher says is dropped, after a plan ran.
    quiet_until: Option<Instant>,
}

/// What the thread that says the changes is told.
enum Told {
    /// A watcher saw these paths change in the git folder it watches.
    Seen {
        /// The git folder watched.
        git_folder: PathBuf,
        /// The paths, as the protocol writes them.
        paths: Vec<String>,
    },
}

impl Entry {
    /// Notes that a change of its git folder was said: what it is now is what the next look compares with.
    fn said(&mut self) {
        self.stamp = git_folder_changed(&self.folder, None).stamp;
    }

    /// Looks at its git folder again. Says whether it changed since it was last looked at with nothing said of it by
    /// its watcher: git writes other files after those that ask for a read, such as the message of a commit, and they
    /// are no change found.
    fn look(&mut self) -> bool {
        let check = git_folder_changed(&self.folder, Some(&self.stamp));
        self.stamp = check.stamp;
        let is_heard = std::mem::take(&mut self.is_heard);
        check.is_anything_changed && !is_heard
    }
}

impl Session {
    /// Watches the git folders of a repository just opened, unless they are watched already, and starts the thread
    /// that says what is seen.
    pub(super) fn watch_git_folders(self: &Arc<Self>, repository: &str, folder: &Path) {
        let mut watched = locked(&self.git_folders.watched);
        if watched.entries.contains_key(repository) {
            return;
        }
        let told = if let Some(told) = &watched.told {
            told.clone()
        } else {
            let (told, receiver) = mpsc::channel();
            let session = Arc::clone(self);
            let spawned = thread::Builder::new()
                .name(String::from("git folders"))
                .spawn(move || say_changes(&session, &receiver));
            if spawned.is_err() {
                // With no thread nothing is said: the extension's own watchers go on as before.
                return;
            }
            watched.told = Some(told.clone());
            told
        };
        let mut folders = Vec::new();
        if let Some(found) = git_folders(folder) {
            if !found.own.starts_with(&found.common) {
                folders.push(found.own);
            }
            folders.push(found.common);
        }
        if std::env::var_os(WATCH_VARIABLE).is_none_or(|value| value != "off") {
            for git_folder in &folders {
                if !watched.watchers.contains_key(git_folder)
                    && let Ok(watcher) = Watcher::new(std::slice::from_ref(git_folder), &telling(git_folder, &told))
                {
                    let _new = watched.watchers.insert(git_folder.clone(), watcher);
                }
            }
        }
        let entry = Entry {
            folder: folder.to_path_buf(),
            git_folders: folders,
            stamp: git_folder_changed(folder, None).stamp,
            is_heard: false,
            is_operating: false,
            held: Vec::new(),
            quiet_until: None,
        };
        let _new = watched.entries.insert(repository.to_owned(), entry);
    }

    /// Stops watching the git folders of a repository closed, but those another repository opened shares.
    pub(super) fn forget_git_folders(&self, repository: &str) {
        let mut watched = locked(&self.git_folders.watched);
        let _closed = watched.entries.remove(repository);
        let Watched { entries, watchers, .. } = &mut *watched;
        watchers.retain(|git_folder, _| entries.values().any(|entry| entry.git_folders.contains(git_folder)));
    }

    /// Notes that an operation of the engine starts in a repository: what is seen in its git folder waits for its end.
    pub fn operation_starts(&self, repository: &str) {
        if let Some(entry) = locked(&self.git_folders.watched).entries.get_mut(repository) {
            entry.is_operating = true;
        }
    }

    /// Notes that the operation of a repository ended. When a plan ran, what was seen meanwhile is the plan's own and
    /// is dropped, with what the watcher says a moment after: the state sent after the plan carries it. Otherwise
    /// what was seen was written by something else while a question waited, and is said now.
    pub fn operation_over(&self, repository: &str, ran: bool) {
        let seen = {
            let mut watched = locked(&self.git_folders.watched);
            let Some(entry) = watched.entries.get_mut(repository) else {
                return;
            };
            entry.is_operating = false;
            let held = std::mem::take(&mut entry.held);
            if ran {
                entry.quiet_until = Some(Instant::now() + QUIET_AFTER_PLAN);
                entry.said();
                return;
            }
            if held.is_empty() {
                return;
            }
            entry.said();
            held
        };
        self.say_git_folder_changed(repository, seen, false);
    }

    /// The answer of `repository/shown`: these repositories are looked at again every two seconds from now on, in
    /// place of those said before, and each of them that was not shown is compared once, now, with what its git folder
    /// was when it was last looked at.
    pub(super) fn shown(&self, params: ShownParams) -> Result<Value, ResponseError> {
        let mut changed = Vec::new();
        {
            let mut watched = locked(&self.git_folders.watched);
            for repository in &params.repositories {
                if watched.shown.contains(repository) || changed.contains(repository) {
                    continue;
                }
                if let Some(entry) = watched.entries.get_mut(repository)
                    && !entry.is_operating
                    && entry.look()
                {
                    changed.push(repository.clone());
                }
            }
            watched.shown = params.repositories;
        }
        json(&ShownResult { changed })
    }

    /// Takes what the watchers saw: each repository of a git folder is said its paths, unless an operation runs
    /// there, for which they wait, or a plan just ran, whose own they are.
    fn git_folders_seen(&self, seen: Vec<(PathBuf, Vec<String>)>) {
        let now = Instant::now();
        let mut said = Vec::new();
        {
            let mut watched = locked(&self.git_folders.watched);
            for (git_folder, paths) in seen {
                for (repository, entry) in &mut watched.entries {
                    if !entry.git_folders.contains(&git_folder) {
                        continue;
                    }
                    entry.is_heard = true;
                    if paths.is_empty() {
                        continue;
                    }
                    if entry.is_operating {
                        add_new(&mut entry.held, &paths);
                    } else if entry.quiet_until.is_none_or(|until| until <= now) {
                        entry.said();
                        said.push((repository.clone(), paths.clone()));
                    }
                }
            }
        }
        for (repository, paths) in said {
            self.say_git_folder_changed(&repository, paths, false);
        }
    }

    /// Looks again at the git folder of each repository shown, and says those that changed with nothing said of it.
    fn look_at_git_folders(&self) {
        let found: Vec<String> = {
            let mut watched = locked(&self.git_folders.watched);
            let Watched { entries, shown, .. } = &mut *watched;
            shown
                .iter()
                .filter(|repository| {
                    entries
                        .get_mut(*repository)
                        .is_some_and(|entry| !entry.is_operating && entry.look())
                })
                .cloned()
                .collect()
        };
        for repository in found {
            self.say_git_folder_changed(&repository, vec![String::from(GIT_FOLDER)], true);
        }
    }

    /// Sends `repository/gitFolderChanged`.
    fn say_git_folder_changed(&self, repository: &str, paths: Vec<String>, found: bool) {
        let params = GitFolderChangedParams {
            repository: repository.to_owned(),
            paths,
            found,
        };
        if let Ok(params) = json(&params) {
            self.send(notification_message(methods::GIT_FOLDER_CHANGED, params));
        }
    }
}

/// Where the watcher of a git folder says what it saw: the paths that ask for a read (`repo.watch-changes`), as the
/// protocol writes them, none when it saw only files that ask for none. A folder whose time was written is left out, since the file created or deleted in it is said
/// on its own; a watcher that lost changes, or whose folder is gone, says the git folder itself.
fn telling(git_folder: &Path, told: &Sender<Told>) -> WatchDeliver {
    let (git_folder, told) = (git_folder.to_path_buf(), told.clone());
    Arc::new(move |events: Vec<WatchEvent>| {
        let mut paths = Vec::new();
        for event in &events {
            if let Some(path) = path_said(&git_folder, event) {
                add_new(&mut paths, &[path]);
            }
        }
        // Also when none asks for a read: the look that follows must know the watcher saw what differs.
        if !events.is_empty() {
            let _sent = told.send(Told::Seen {
                git_folder: git_folder.clone(),
                paths,
            });
        }
    })
}

/// What a change the system reported in a git folder is said as; none when it asks for no read.
fn path_said(git_folder: &Path, event: &WatchEvent) -> Option<String> {
    let inside = match event {
        WatchEvent::Changed(path) if path.is_dir() => return None,
        WatchEvent::Changed(path) => path.strip_prefix(git_folder).ok()?.to_path_buf(),
        WatchEvent::Overflowed(_) | WatchEvent::Gone(_) => PathBuf::new(),
    };
    let _read = watch_changes(&Change::GitFolder(inside.clone()))?;
    let names: Vec<String> = inside.iter().map(|name| name.to_string_lossy().into_owned()).collect();
    if names.is_empty() {
        return Some(String::from(GIT_FOLDER));
    }
    Some(format!("{GIT_FOLDER}/{}", names.join("/")))
}

/// Adds the paths that are not there yet.
fn add_new(paths: &mut Vec<String>, more: &[String]) {
    for path in more {
        if !paths.contains(path) {
            paths.push(path.clone());
        }
    }
}

/// What the thread does until the engine is shut down: it says what the watchers saw as it comes, and looks again at
/// the repositories shown every two seconds.
fn say_changes(session: &Arc<Session>, receiver: &Receiver<Told>) {
    let mut next = Instant::now() + LOOK_INTERVAL;
    while !session.is_shut_down() {
        match receiver.recv_timeout(next.saturating_duration_since(Instant::now())) {
            Ok(first) => {
                // Many changes said at once are taken together: each git folder is looked at once for them.
                let mut seen: Vec<(PathBuf, Vec<String>)> = Vec::new();
                for Told::Seen { git_folder, paths } in std::iter::once(first).chain(receiver.try_iter()) {
                    match seen.iter_mut().find(|(folder, _)| *folder == git_folder) {
                        Some((_, together)) => add_new(together, &paths),
                        None => seen.push((git_folder, paths)),
                    }
                }
                session.git_folders_seen(seen);
            }
            Err(RecvTimeoutError::Timeout) => {
                next = Instant::now() + LOOK_INTERVAL;
                session.look_at_git_folders();
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use system::WatchEvent;

    use super::path_said;

    #[test]
    fn what_a_watcher_saw_is_said_as_repository_changed_writes_it_and_what_asks_for_no_read_is_not() {
        let git_folder = Path::new("/work/app/.git");
        let said = |path: &str| path_said(git_folder, &WatchEvent::Changed(git_folder.join(path)));
        assert_eq!(said("refs/heads/main"), Some(String::from(".git/refs/heads/main")));
        assert_eq!(
            said("worktrees/feature/HEAD"),
            Some(String::from(".git/worktrees/feature/HEAD"))
        );
        assert_eq!(said("index.lock"), None);
        assert_eq!(said("objects/ab/cdef"), None);
        assert_eq!(said("worktrees/feature/fsmonitor--daemon/cookies/1"), None);
        let lost = path_said(git_folder, &WatchEvent::Overflowed(git_folder.to_path_buf()));
        assert_eq!(lost, Some(String::from(".git")));
        let elsewhere = path_said(git_folder, &WatchEvent::Changed(PathBuf::from("/elsewhere/HEAD")));
        assert_eq!(elsewhere, None);
    }
}
