//! The workers of a window: a small pool of threads that run the jobs the store asks for, each with a token that
//! cancels it and an entry among the activities, and send back the message of its result.

pub mod compare;
pub mod diff;
pub mod file_view;
pub mod jobs;
pub mod merge_view;
mod operations;
mod persist;
mod repository_watch;
pub mod review;
mod setting_writer;
mod watch;

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

use engine::Activities;
use git::CancelToken;
use model::Timestamp;
use model::contract::Text;

use views::text::activity;
use views::text::search as search_words;

use crate::store::search::SearchJob;
use crate::store::{Job, JobId, JobKind, Message};

pub use operations::Operations;
pub use persist::PersistedWriter;
pub use repository_watch::{RepositoryWatch, change_of, common_git_folder};
pub use setting_writer::SettingWriter;
pub use watch::{SettingsWatcher, WATCH_INTERVAL};

/// Where the messages of the workers go: into the event loop of the window, which it wakes up.
pub type Deliver = Arc<dyn Fn(Message) + Send + Sync>;

/// A job given to a worker, with its token.
struct Task {
    /// The job.
    job: Job,
    /// The token that cancels it.
    cancel: CancelToken,
    /// What it is and why it runs, in the words of the Activity drawer, when the window says them; otherwise they are
    /// made from the job.
    words: Option<(Text, Text)>,
}

/// The pool of workers of a window.
pub struct Workers {
    /// Where the jobs go; none once the pool is closing.
    sender: Option<Sender<Task>>,
    /// The threads.
    threads: Vec<JoinHandle<()>>,
    /// The token of each job not finished.
    tokens: HashMap<JobId, CancelToken>,
}

impl Workers {
    /// A pool of threads, which put each job among the activities while it runs and give its result to `deliver`.
    #[must_use]
    pub fn new(count: usize, activities: &Activities, deliver: &Deliver) -> Workers {
        let (sender, receiver) = mpsc::channel::<Task>();
        let receiver = Arc::new(Mutex::new(receiver));
        let threads = (0..count.max(1))
            .filter_map(|number| {
                let receiver = Arc::clone(&receiver);
                let activities = activities.clone();
                let deliver = Arc::clone(deliver);
                thread::Builder::new()
                    .name(format!("worker {number}"))
                    .spawn(move || work(&receiver, &activities, &deliver))
                    .ok()
            })
            .collect();
        Workers {
            sender: Some(sender),
            threads,
            tokens: HashMap::new(),
        }
    }

    /// Gives a job to the next worker free.
    pub fn start(&mut self, job: Job) {
        self.start_described(job, None);
    }

    /// Gives a job to the next worker free, with what it is and why it runs in the words of the Activity drawer, such
    /// as `Reading app` and `Because src/a.rs changed.` (`activity.describe-task`).
    pub fn start_described(&mut self, job: Job, words: Option<(Text, Text)>) {
        let cancel = CancelToken::new();
        let _replaced = self.tokens.insert(job.id, cancel.clone());
        if let Some(sender) = &self.sender {
            let _sent = sender.send(Task { job, cancel, words });
        }
    }

    /// Cancels a job: the git it runs is stopped, and its result, if any, is not wanted.
    pub fn cancel(&mut self, id: JobId) {
        if let Some(token) = self.tokens.remove(&id) {
            token.cancel();
        }
    }

    /// Forgets a job whose result came back.
    pub fn finished(&mut self, id: JobId) {
        let _done = self.tokens.remove(&id);
    }
}

impl Drop for Workers {
    fn drop(&mut self) {
        for token in self.tokens.values() {
            token.cancel();
        }
        self.sender = None;
        for thread in self.threads.drain(..) {
            let _ended = thread.join();
        }
    }
}

/// What a worker does until the pool closes: takes a job, runs it as an activity, delivers its result.
fn work(receiver: &Mutex<Receiver<Task>>, activities: &Activities, deliver: &Deliver) {
    loop {
        // The lock is held only to take the next job; a worker that panicked left the receiver whole.
        let task = receiver.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let Ok(task) = task else {
            return;
        };
        if task.cancel.is_cancelled() {
            continue;
        }
        let message = {
            let (description, cause) = task.words.clone().unwrap_or_else(|| words(&task.job.kind));
            let _activity = activities.track(description, cause, now());
            jobs::run(&task.job, &task.cancel)
        };
        if !task.cancel.is_cancelled() {
            deliver(message);
        }
    }
}

/// What a job is and what asked for it, in the words of the Activity drawer.
fn words(kind: &JobKind) -> (Text, Text) {
    let (folder, cause) = match kind {
        JobKind::FindRepositories { folders } => {
            let names: Vec<String> = folders.iter().map(|folder| folder_name(folder)).collect();
            return (
                Text::from(activity::finding_repositories(&names.join(", "))),
                Text::from(activity::OPENING_WORKSPACE),
            );
        }
        JobKind::Search(search) => {
            let cause = match search {
                SearchJob::Matches { .. } => search_words::SEARCHING_HISTORY,
                SearchJob::Revision { .. } | SearchJob::Position { .. } => search_words::LOOKING_FOR_COMMIT,
            };
            return (
                Text::from(search_words::searching(&folder_name(search.folder()))),
                Text::from(cause),
            );
        }
        JobKind::ReadRepository { folder, .. } => (folder, activity::SHOWING_REPOSITORY),
        JobKind::ReadCommits { folder, .. } => (folder, activity::LOADING_MORE_COMMITS),
        JobKind::ReadTimeline { folder, .. } => (folder, activity::NARROWING_TIMELINE),
        JobKind::ReadDetails { folder, .. } => (folder, activity::SHOWING_DETAILS),
        JobKind::ReadCommitPage { folder, .. } => (folder, activity::OPENING_COMMIT_PAGE),
        JobKind::ReadDiff { folder, .. } => (folder, activity::COMPUTING_DIFF),
        JobKind::ReadFile { folder, .. } => (folder, activity::OPENING_FILE),
        JobKind::ReadComparison { folder, .. } | JobKind::ReadInPlace { folder, .. } => (folder, activity::COMPARING),
        JobKind::CountConflicts { folder } => (folder, activity::COUNTING_CONFLICTS),
        JobKind::ReadMerge { folder, .. } => (folder, activity::SHOWING_MERGE),
        JobKind::ReadReview { folder, .. } => (folder, activity::REVIEWING_CONFLICTS),
    };
    (Text::from(activity::reading(&folder_name(folder))), Text::from(cause))
}

/// The last name of a folder, or the whole folder when it has none.
fn folder_name(folder: &Path) -> String {
    folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The time now, in seconds since 1970.
fn now() -> Timestamp {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(i64::MAX));
    Timestamp(seconds)
}
