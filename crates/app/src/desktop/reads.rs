//! What the window does for the reads of the repository shown (`repo.watch-changes`, `repo.schedule-reads`): the
//! watcher of the repository, the timer of the next scheduled read, the focus and the minimized window told to the
//! store, the words of each read in the Activity drawer, and how long each took, for the Command log.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use model::Timestamp;
use model::activity::{Task, TaskKind, describe_task};
use model::contract::Text;
use model::repo::{Change, name_repositories};
use platform::{Context, InputEvent, TimerId};

use crate::store::{Job, JobId, JobKind, Message, State};
use crate::work::{Deliver, RepositoryWatch};

/// The timer of the next scheduled read.
pub const READ_TIMER: TimerId = TimerId(2);
/// The least a timer waits.
const LEAST_WAIT: Duration = Duration::from_millis(1);

/// What the window keeps for the reads of the repository shown.
pub struct ReadClock {
    /// When the window opened: the moments of the schedule are counted from it.
    pub origin: Instant,
    /// The watcher of the repository shown; none while none is shown, or when the system cannot watch it.
    watch: Option<RepositoryWatch>,
    /// The moment the store asked to be woken at, until the timer is set.
    wake_at: Option<Duration>,
    /// Whether the window is minimized.
    is_minimized: bool,
    /// When each read of a repository started, with its line in the Command log.
    started: HashMap<JobId, (Instant, Timestamp, String)>,
}

impl ReadClock {
    /// The clock of a window opened now.
    #[must_use]
    pub fn new() -> ReadClock {
        ReadClock {
            origin: Instant::now(),
            watch: None,
            wake_at: None,
            is_minimized: false,
            started: HashMap::new(),
        }
    }

    /// Watches the working tree of the repository shown, in place of the one before.
    pub fn watch(&mut self, folder: Option<&Path>, deliver: &Deliver) {
        self.watch = None;
        self.watch = folder.and_then(|folder| RepositoryWatch::new(folder, self.origin, deliver));
    }

    /// Notes the moment to wake the store at; the timer is set once the loop can be asked.
    pub fn wake_at(&mut self, at: Duration) {
        self.wake_at = Some(at);
    }

    /// Sets the timer of the wake asked for, if one was.
    pub fn arm(&mut self, context: &Context<'_>) {
        if let Some(at) = self.wake_at.take() {
            let wait = at.saturating_sub(self.origin.elapsed()).max(LEAST_WAIT);
            context.start_timer(READ_TIMER, wait);
        }
    }

    /// The timer of the next read went off: it is stopped, and the store told the moment.
    pub fn woken(&self, context: &Context<'_>) -> Message {
        context.stop_timer(READ_TIMER);
        Message::Wake {
            at: self.origin.elapsed(),
        }
    }

    /// What input for the window tells the store: the focus gained asks for a read, a window made as small as nothing
    /// is minimized.
    pub fn input(&mut self, event: &InputEvent) -> Option<Message> {
        match event {
            InputEvent::FocusGained => Some(Message::Changed {
                changes: vec![Change::Focus],
                at: self.origin.elapsed(),
            }),
            InputEvent::Resized(size) => {
                let is_minimized = size.width == 0 || size.height == 0;
                (is_minimized != self.is_minimized).then(|| {
                    self.is_minimized = is_minimized;
                    Message::Minimized(is_minimized)
                })
            }
            _ => None,
        }
    }

    /// Notes a job started; for a read of a repository the schedule counts, gives what it is and why it runs, in the
    /// words of the Activity drawer.
    pub fn started(&mut self, state: &State, job: &Job, now: Timestamp) -> Option<(Text, Text)> {
        let JobKind::ReadRepository { folder, request } = &job.kind else {
            return None;
        };
        let line = format!(
            "read {}: the state and {} pages of commits",
            slashes(folder),
            request.pages
        );
        let _replaced = self.started.insert(job.id, (Instant::now(), now, line));
        let reason = state.reads.reason(job.id)?;
        let task = Task {
            kind: TaskKind::Read {
                name: repository_name(state, folder),
                is_search: is_search_in_changes(&request.narrowing),
                cause: Some(reason.cause.clone()),
                in_a_row: reason.in_a_row,
            },
            started: Instant::now(),
        };
        let text = describe_task(&task, task.started);
        Some((Text::from(text.title), Text::from(text.detail.unwrap_or_default())))
    }

    /// A read of a repository ended: its line in the Command log, with how long it took.
    pub fn finished(&mut self, job: JobId) -> Option<Message> {
        let (started, time, line) = self.started.remove(&job)?;
        Some(Message::ReadTook {
            line,
            took: started.elapsed(),
            time,
        })
    }
}

/// The name of a repository, as `repo.name-repositories` gives it.
fn repository_name(state: &State, folder: &Path) -> String {
    let roots: Vec<PathBuf> = state
        .workspace
        .as_ref()
        .map(|workspace| workspace.folders.clone())
        .unwrap_or_default();
    name_repositories(&roots, &[folder.to_path_buf()])
        .into_iter()
        .next()
        .unwrap_or_else(|| slashes(folder))
}

/// A path with forward slashes, as the log writes paths on every system.
fn slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Whether a search in the changes, `git log -S` or `-G`, narrows a read (`search.list-mode`).
fn is_search_in_changes(narrowing: &[std::ffi::OsString]) -> bool {
    narrowing.iter().any(|argument| {
        let argument = argument.to_string_lossy();
        argument.starts_with("-S") || argument.starts_with("-G")
    })
}
