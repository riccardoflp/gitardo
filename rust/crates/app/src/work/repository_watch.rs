//! The watcher of the repository shown (`repo.watch-changes`), and of those of the tabs not shown
//! (`repo.watch-hidden`): its working tree, and its common git folder when that is outside it, as a worktree's is. Each batch of changes the system reports goes to the store as one message, with
//! the paths made relative to the folder they are in; the store decides which ask for a read. A folder whose time was
//! written is left out: the files created or deleted in it are said on their own.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use model::repo::Change;
use platform::{WatchDeliver, WatchEvent, Watcher};

use super::Deliver;
use crate::store::Message;

/// The folders of a repository watched, until it is dropped.
pub struct RepositoryWatch {
    /// The watcher of the platform, which stops its threads when dropped.
    _watcher: Watcher,
}

impl RepositoryWatch {
    /// Watches a working tree and its common git folder; the moments of the changes are counted from `origin`, the
    /// moment the window opened. None when the system cannot watch them.
    #[must_use]
    pub fn new(tree: &Path, origin: Instant, deliver: &Deliver) -> Option<RepositoryWatch> {
        RepositoryWatch::saying(tree, (origin, deliver), |changes, at| Message::Changed { changes, at })
    }

    /// Watches the working tree of a tab not shown and its common git folder: its changes are said with its folder.
    #[must_use]
    pub fn for_tab(tree: &Path, origin: Instant, deliver: &Deliver) -> Option<RepositoryWatch> {
        let folder = tree.to_path_buf();
        RepositoryWatch::saying(tree, (origin, deliver), move |changes, at| Message::TabChanged {
            folder: folder.clone(),
            changes,
            at,
        })
    }

    /// Watches a working tree and its common git folder, each batch of changes said as a message makes it.
    fn saying(
        tree: &Path,
        (origin, deliver): (Instant, &Deliver),
        message: impl Fn(Vec<Change>, Duration) -> Message + Send + Sync + 'static,
    ) -> Option<RepositoryWatch> {
        let git_folder = common_git_folder(tree);
        let mut folders = vec![tree.to_path_buf()];
        if let Some(git_folder) = git_folder.as_ref().filter(|git_folder| !git_folder.starts_with(tree)) {
            folders.push(git_folder.clone());
        }
        let deliver = Arc::clone(deliver);
        let tree = tree.to_path_buf();
        let changes: WatchDeliver = Arc::new(move |events: Vec<WatchEvent>| {
            let changes: Vec<Change> = events
                .iter()
                .filter(|event| !is_folder_written(event))
                .filter_map(|event| change_of(&tree, git_folder.as_deref(), event))
                .collect();
            if !changes.is_empty() {
                deliver(message(changes, origin.elapsed()));
            }
        });
        Watcher::new(&folders, &changes)
            .ok()
            .map(|watcher| RepositoryWatch { _watcher: watcher })
    }
}

/// Whether a change is only the time a folder was written: the system says it when a file in the folder is created or
/// deleted, which it says too, and sometimes long after, when it writes the times of the folder down. It asks for no
/// read of its own.
fn is_folder_written(event: &WatchEvent) -> bool {
    matches!(event, WatchEvent::Changed(path) if path.is_dir())
}

/// A change the system reported, as the store takes it: in the git folder, relative to it; in the working tree,
/// relative to it. A folder that reported more than it could say, or that is gone, is a change of the whole folder.
#[must_use]
pub fn change_of(tree: &Path, git_folder: Option<&Path>, event: &WatchEvent) -> Option<Change> {
    let path = match event {
        WatchEvent::Changed(path) | WatchEvent::Overflowed(path) | WatchEvent::Gone(path) => path,
    };
    if let Some(relative) = git_folder.and_then(|git_folder| path.strip_prefix(git_folder).ok()) {
        return Some(Change::GitFolder(relative.to_path_buf()));
    }
    let relative = path.strip_prefix(tree).ok()?;
    if relative.as_os_str().is_empty() {
        return Some(Change::WorkingTree(PathBuf::from(".")));
    }
    Some(Change::WorkingTree(relative.to_path_buf()))
}

/// The common git folder of a working tree: `.git` when it is a folder; for a worktree or a submodule, whose `.git` is
/// a file naming its own git folder, the folder that one's `commondir` names, or its own git folder.
#[must_use]
pub fn common_git_folder(tree: &Path) -> Option<PathBuf> {
    let dot_git = tree.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let named = text.trim().strip_prefix("gitdir:")?.trim();
    let own = tree.join(named);
    let common = fs::read_to_string(own.join("commondir"))
        .ok()
        .map_or_else(|| own.clone(), |common| own.join(common.trim()));
    Some(fs::canonicalize(&common).map_or(common, strip_verbatim))
}

/// A path the system made whole, without the `\\?\` it puts before it on Windows, so that it compares with the paths
/// of the working tree.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}
