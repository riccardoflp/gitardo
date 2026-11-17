//! The files of the working tree that changed, asked of git before they ask for a read (`repo.watch-changes`, M15): a
//! file in the folder of another worktree nested in the one shown asks for nothing; the others of a burst are given to
//! `git check-ignore` together, a second after the first of them changed, and only those git does not ignore ask for a
//! read. A `.gitignore` asks for one at once.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use model::repo::{LATER, TreeChange, nested_worktrees, sort_tree_change};

use super::message::{Effect, Job, JobId, JobKind};
use super::state::State;

/// The files of the working tree that changed and wait for git to say whether it ignores them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ChangedFiles {
    /// The files not asked of git yet, from the top of the working tree with `/`.
    waiting: BTreeSet<String>,
    /// When the first of them changed.
    since: Option<Duration>,
    /// The question to git under way.
    asking: Option<Asking>,
}

/// A question to git under way: which of some files it ignores.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Asking {
    /// Its job.
    job: JobId,
    /// The files asked.
    paths: Vec<String>,
    /// When the first of them changed.
    since: Duration,
}

impl ChangedFiles {
    /// The moment the files waiting are asked of git: a second after the first of them changed, once no question is
    /// under way. None while nothing waits.
    #[must_use]
    pub fn due_at(&self) -> Option<Duration> {
        self.since
            .filter(|_| self.asking.is_none() && !self.waiting.is_empty())
            .map(|since| since + LATER)
    }

    /// Whether a job is the question to git under way.
    #[must_use]
    pub fn is_asking(&self, job: JobId) -> bool {
        self.asking.as_ref().is_some_and(|asking| asking.job == job)
    }

    /// Forgets every file changed until now: a read starts, which sees them. The answer of a question under way is not
    /// waited for any more.
    pub fn read_started(&mut self) {
        *self = ChangedFiles::default();
    }
}

/// Sorts a change of a file of the working tree of the repository shown: whether it asks for a read at once, or for
/// nothing now, either because it waits to be asked of git or because it is in a nested worktree.
pub fn sort(state: &mut State, path: &Path, at: Duration) -> TreeChange {
    let nested = state
        .shown_read()
        .map(|read| nested_worktrees(&read.state))
        .unwrap_or_default();
    let sorted = sort_tree_change(path, &nested);
    if sorted == TreeChange::AskGit {
        let files = &mut state.reads.files;
        let _new = files.waiting.insert(slashed(path));
        files.since = files.since.or(Some(at));
    }
    sorted
}

/// The moment came: the files waiting are asked of git, when their second is over and no question is under way.
pub fn wake(state: &mut State, at: Duration) -> Option<Effect> {
    let due = state.reads.files.due_at().filter(|due| *due <= at)?;
    let folder = state.shown_read().map(|read| read.folder.clone())?;
    let files = &mut state.reads.files;
    let paths: Vec<String> = std::mem::take(&mut files.waiting).into_iter().collect();
    let since = files.since.take().unwrap_or(due);
    let id = JobId(state.next_job);
    state.next_job += 1;
    let kind = JobKind::ReadIgnored {
        folder,
        paths: paths.clone(),
    };
    let _new = state.jobs.insert(id, kind.clone());
    state.reads.files.asking = Some(Asking { job: id, paths, since });
    Some(Effect::Start(Job { id, kind }))
}

/// Git said which of the files asked it ignores: the others, and when the first of them changed. When git could not
/// say, as when it failed, every file asked is taken as one it does not ignore, so that no change is missed. None
/// for the answer of a question no longer waited for.
pub fn answered(
    state: &mut State,
    job: JobId,
    result: Result<Vec<String>, String>,
) -> Option<(Vec<PathBuf>, Duration)> {
    let _finished = state.jobs.remove(&job);
    let asking = state.reads.files.asking.take_if(|asking| asking.job == job)?;
    let ignored: BTreeSet<String> = result.unwrap_or_default().into_iter().collect();
    let kept = asking
        .paths
        .into_iter()
        .filter(|path| !ignored.contains(path))
        .map(PathBuf::from)
        .collect();
    Some((kept, asking.since))
}

/// A path from the top of the working tree with `/`, as git takes it on its standard input.
fn slashed(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
