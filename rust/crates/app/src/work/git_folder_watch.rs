//! Looking again at the git folder of the repository shown (`repo.watch-git`, D93): a thread that compares, every two
//! seconds, what [`git_folder_changed`] looks at with what it was, and says a change of the git folder when something
//! differs that the watcher said nothing of. A watcher can lose a change, start late, or say nothing on some disks; a
//! commit, a branch or a worktree made outside is then still shown within three seconds.
//!
//! A change the watcher said is looked at as soon as it is said, so that the comparison does not find it again and ask
//! for a second read. The git folder of a tab not shown is not looked at every two seconds: what it was is kept, and it
//! is compared once when the tab is shown again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use git::read::worktree::{GitFolderStamp, git_folder_changed};
use model::repo::Change;

use super::Deliver;
use crate::store::Message;

/// How often the git folder of the repository shown is looked at: a change its watcher lost is shown within this, and
/// the time a read takes (Q1792).
pub const LOOK_INTERVAL: Duration = Duration::from_secs(2);

/// What the thread is told.
enum Told {
    /// This working tree is shown, in place of the one before; none when no repository is.
    Show(Option<PathBuf>),
    /// The watcher of this working tree said something changed in its git folder.
    Said(PathBuf),
    /// These are the working trees of the tabs not shown: what the git folders of the others were is forgotten.
    Keep(Vec<PathBuf>),
}

/// Where a watcher says that it saw the git folder of a working tree change.
#[derive(Clone)]
pub struct GitFolderSaid {
    /// Where the thread that looks again is told.
    sender: Sender<Told>,
}

impl GitFolderSaid {
    /// Notes that the watcher said a change of the git folder of this working tree: it is not found again.
    pub fn said(&self, tree: &Path) {
        let _sent = self.sender.send(Told::Said(tree.to_path_buf()));
    }
}

/// The thread that looks again at the git folder of the repository shown, until it is dropped.
pub struct GitFolderWatch {
    /// Where the thread is told; none once it is closing.
    sender: Option<Sender<Told>>,
    /// The thread.
    thread: Option<JoinHandle<()>>,
}

impl GitFolderWatch {
    /// A thread that looks at nothing yet; what it finds goes to `deliver` as a change of the repository shown, at a
    /// moment counted from `origin`.
    #[must_use]
    pub fn new(origin: Instant, deliver: Deliver) -> GitFolderWatch {
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(String::from("git folder watcher"))
            .spawn(move || look_again(&receiver, origin, &deliver))
            .ok();
        GitFolderWatch {
            sender: Some(sender),
            thread,
        }
    }

    /// Looks at the git folder of this working tree from now on, in place of the one before. When it was shown before
    /// and something changed in it since, with nothing said, that is said at once.
    pub fn show(&self, tree: Option<&Path>) {
        self.tell(Told::Show(tree.map(Path::to_path_buf)));
    }

    /// Keeps what the git folders of these working trees were, those of the tabs not shown, and forgets the others.
    pub fn keep(&self, trees: &[PathBuf]) {
        self.tell(Told::Keep(trees.to_vec()));
    }

    /// Where the watchers say what they saw.
    #[must_use]
    pub fn said(&self) -> Option<GitFolderSaid> {
        self.sender.clone().map(|sender| GitFolderSaid { sender })
    }

    /// Tells the thread something.
    fn tell(&self, told: Told) {
        if let Some(sender) = &self.sender {
            let _sent = sender.send(told);
        }
    }
}

impl Drop for GitFolderWatch {
    fn drop(&mut self) {
        self.sender = None;
        if let Some(thread) = self.thread.take() {
            let _ended = thread.join();
        }
    }
}

/// What the git folders looked at were when last looked at.
#[derive(Debug, Default)]
pub struct Looks {
    /// The working tree shown.
    shown: Option<PathBuf>,
    /// What the git folder of each working tree was: the one shown, and those kept for the tabs not shown.
    stamps: BTreeMap<PathBuf, GitFolderStamp>,
}

impl Looks {
    /// Shows a working tree in place of the one before. Says whether its git folder changed since it was last looked
    /// at, which only a working tree shown before can say.
    pub fn show(&mut self, tree: Option<PathBuf>) -> bool {
        self.shown = tree;
        self.look()
    }

    /// Notes that the watcher said a change in the git folder of a working tree looked at: what it is now is what the
    /// next look compares with.
    pub fn said(&mut self, tree: &Path) {
        if let Some(stamp) = self.stamps.get_mut(tree) {
            *stamp = git_folder_changed(tree, None).stamp;
        }
    }

    /// Forgets what the git folders were, but those of the working tree shown and of these.
    pub fn keep(&mut self, trees: &[PathBuf]) {
        let shown = self.shown.clone();
        self.stamps
            .retain(|tree, _| shown.as_ref() == Some(tree) || trees.contains(tree));
    }

    /// Looks at the git folder of the working tree shown. Says whether it changed since it was last looked at; the
    /// first look at a working tree says it did not.
    pub fn look(&mut self) -> bool {
        let Some(tree) = self.shown.clone() else {
            return false;
        };
        let before = self.stamps.get(&tree);
        let check = git_folder_changed(&tree, before);
        let is_changed = before.is_some() && check.is_anything_changed;
        let _before = self.stamps.insert(tree, check.stamp);
        is_changed
    }
}

/// What the thread does until the watch is dropped.
fn look_again(receiver: &Receiver<Told>, origin: Instant, deliver: &Deliver) {
    let mut looks = Looks::default();
    let mut next = Instant::now() + LOOK_INTERVAL;
    loop {
        let wait = next.saturating_duration_since(Instant::now());
        let is_changed = match receiver.recv_timeout(wait) {
            Ok(told) => take(&mut looks, told, receiver),
            Err(RecvTimeoutError::Timeout) => {
                next = Instant::now() + LOOK_INTERVAL;
                looks.look()
            }
            Err(RecvTimeoutError::Disconnected) => return,
        };
        if is_changed {
            deliver(Message::Changed {
                changes: vec![Change::GitFolder(PathBuf::new())],
                at: origin.elapsed(),
            });
        }
    }
}

/// Takes what the thread was told, and everything told since: many changes said at once are looked at once. Says
/// whether a working tree just shown changed while it was not.
fn take(looks: &mut Looks, first: Told, receiver: &Receiver<Told>) -> bool {
    let mut said: Vec<PathBuf> = Vec::new();
    let mut is_changed = false;
    for told in std::iter::once(first).chain(receiver.try_iter()) {
        match told {
            Told::Show(tree) => is_changed = looks.show(tree),
            Told::Keep(trees) => looks.keep(&trees),
            Told::Said(tree) => {
                if !said.contains(&tree) {
                    said.push(tree);
                }
            }
        }
    }
    for tree in said {
        looks.said(&tree);
    }
    is_changed
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::Looks;

    #[test]
    fn repo_watch_git__a_commit_nobody_said_is_found_once_and_one_the_watcher_said_is_not() {
        let mut repository = testkit::Repository::new();
        let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
        let tree = repository.path().to_path_buf();
        let mut looks = Looks::default();
        assert!(!looks.show(Some(tree.clone())), "the first look finds nothing");
        assert!(!looks.look(), "nothing was written");
        let _second = repository.commit("Add b", &[("b.txt", "b\n")]);
        assert!(looks.look(), "nobody said the commit");
        assert!(!looks.look(), "it is found once");
        let _third = repository.commit("Add c", &[("c.txt", "c\n")]);
        looks.said(&tree);
        assert!(!looks.look(), "the watcher said it");
    }

    #[test]
    fn repo_watch_git__a_tab_not_shown_is_compared_once_when_it_is_shown_again() {
        let mut first = testkit::Repository::new();
        let _commit = first.commit("Add a", &[("a.txt", "a\n")]);
        let mut second = testkit::Repository::new();
        let _commit = second.commit("Add a", &[("a.txt", "a\n")]);
        let (first_tree, second_tree) = (first.path().to_path_buf(), second.path().to_path_buf());
        let mut looks = Looks::default();
        assert!(!looks.show(Some(first_tree.clone())));
        assert!(!looks.show(Some(second_tree.clone())));
        looks.keep(std::slice::from_ref(&first_tree));
        let _made = first.git(&["branch", "feature"]);
        assert!(!looks.look(), "the tab not shown is not looked at");
        assert!(
            looks.show(Some(first_tree.clone())),
            "it changed while it was not shown"
        );
        assert!(!looks.show(Some(second_tree.clone())), "nothing changed in the other");
        let _made = second.git(&["tag", "v1"]);
        looks.said(&second_tree);
        looks.keep(std::slice::from_ref(&second_tree));
        let _back = looks.show(Some(first_tree));
        assert!(!looks.show(Some(second_tree.clone())), "its watcher said the tag");
        looks.keep(&[]);
        let _away = looks.show(None);
        looks.keep(&[]);
        assert!(!looks.show(Some(second_tree)), "a tab closed is looked at as new");
    }
}
