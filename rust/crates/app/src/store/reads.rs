//! The reads of the repository shown that nobody asked for by hand (`repo.watch-changes`, `repo.schedule-reads`):
//! the changes the watcher sees become reads after their delay, one at a time, and the reads the window starts itself
//! (a repository chosen, the timeline read again for a switch) are counted by the same schedule, so that a read of a
//! change never runs beside them.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use model::Timestamp;
use model::repo::{Change, ReadAction, ReadCause, ReadEvent, ReadSchedule, StartedRead, schedule_reads, watch_changes};

use super::message::{Effect, Job, JobId, JobKind};
use super::state::State;

/// What the window knows of the reads of the repository shown.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Reads {
    /// The schedule.
    pub schedule: ReadSchedule,
    /// The reads of the repository running, by their job: their number in the schedule, why each runs and how many
    /// reads in a row it is.
    pub scheduled: BTreeMap<JobId, StartedRead>,
    /// The working tree whose changes are watched; none while no repository is read.
    pub watched: Option<PathBuf>,
    /// Whether the window opened with `readOnStartup` off and its repository is not read yet: the timeline says **Not
    /// read yet** with **Read the repository**.
    pub is_not_read: bool,
}

impl Reads {
    /// Why a read of the repository runs, and how many reads in a row it is, when the schedule started it or counts
    /// it.
    #[must_use]
    pub fn reason(&self, job: JobId) -> Option<&StartedRead> {
        self.scheduled.get(&job)
    }
}

/// Changes seen by the watcher of the repository, at a moment counted from when the window opened: each change that
/// counts asks for a read after its delay. Nothing is read while no repository is shown.
pub fn changed(state: &mut State, changes: &[Change], at: Duration) -> Vec<Effect> {
    if state.shown_read().is_none() {
        return Vec::new();
    }
    let mut effects = Vec::new();
    for change in changes {
        if let Some(request) = watch_changes(change) {
            effects.extend(tell(state, &ReadEvent::Changed { request, at }));
        }
    }
    dedupe_wakes(effects)
}

/// The moment a wake asked for came.
pub fn wake(state: &mut State, at: Duration) -> Vec<Effect> {
    tell(state, &ReadEvent::Tick { at })
}

/// A read asked for at once, such as for settings that change what is read.
pub fn asked(state: &mut State, cause: ReadCause) -> Vec<Effect> {
    if state.shown_read().is_none() {
        return Vec::new();
    }
    tell(state, &ReadEvent::Asked(cause))
}

/// The window was minimized, or shown again.
pub fn minimized(state: &mut State, is_minimized: bool) -> Vec<Effect> {
    tell(state, &ReadEvent::Minimized(is_minimized))
}

/// A read the window started itself, after cancelling the reads running: the schedule forgets the read it counted on
/// and counts this one, so that the reads of changes wait for it.
pub fn adopt(state: &mut State, job: JobId, cause: ReadCause) {
    let (schedule, actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Restarted(cause));
    state.reads.schedule = schedule;
    for action in actions {
        match action {
            ReadAction::Start(started) => {
                let _counted = state.reads.scheduled.insert(job, started);
            }
            ReadAction::Discard(number) => {
                state.reads.scheduled.retain(|_, read| read.number != number);
            }
            ReadAction::WakeAt(_) | ReadAction::Show(_) | ReadAction::StopBar => {}
        }
    }
}

/// The reads running were cancelled, and nothing replaces them.
pub fn stopped(state: &mut State) {
    let (schedule, _actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Stopped);
    state.reads.schedule = schedule;
    state.reads.scheduled.clear();
}

/// A read of the repository ended: whether what it found is shown, and the read that follows it, if one does.
pub fn finished(state: &mut State, job: JobId) -> (bool, Vec<Effect>) {
    let Some(read) = state.reads.scheduled.remove(&job) else {
        return (true, Vec::new());
    };
    let (schedule, actions) = schedule_reads(&state.reads.schedule, &ReadEvent::Finished(read.number));
    state.reads.schedule = schedule;
    let is_shown = !actions.contains(&ReadAction::Discard(read.number));
    let effects = carry_out(state, actions);
    (is_shown, effects)
}

/// The working tree to watch now, when it is not the one watched: the repository shown once read, none otherwise.
pub fn watch(state: &mut State) -> Option<Effect> {
    let folder = state.shown_read().map(|read| read.folder.clone());
    if folder == state.reads.watched {
        return None;
    }
    state.reads.watched.clone_from(&folder);
    Some(Effect::WatchRepository(folder))
}

/// The debug line of a read of the repository that took a while, in the Command log at `log.level` `debug`.
pub fn read_took(state: &mut State, (line, took, time): (String, Duration, Timestamp)) {
    state.log.record_read(time, line, took);
}

/// Tells the schedule an event and carries out what it asks.
fn tell(state: &mut State, event: &ReadEvent) -> Vec<Effect> {
    let (schedule, actions) = schedule_reads(&state.reads.schedule, event);
    state.reads.schedule = schedule;
    carry_out(state, actions)
}

/// What the schedule asked: reads started, reads thrown away, wakes asked for.
fn carry_out(state: &mut State, actions: Vec<ReadAction>) -> Vec<Effect> {
    let mut effects = Vec::new();
    for action in actions {
        match action {
            ReadAction::Start(started) => effects.extend(start(state, started)),
            ReadAction::Discard(number) => {
                let jobs: Vec<JobId> = state
                    .reads
                    .scheduled
                    .iter()
                    .filter(|(_, read)| read.number == number)
                    .map(|(job, _)| *job)
                    .collect();
                for job in jobs {
                    let _forgotten = state.reads.scheduled.remove(&job);
                    if state.jobs.remove(&job).is_some() {
                        effects.push(Effect::Cancel(job));
                    }
                }
            }
            ReadAction::WakeAt(at) => effects.push(Effect::WakeAt(at)),
            ReadAction::Show(_) | ReadAction::StopBar => {}
        }
    }
    effects
}

/// Starts a read the schedule asked for: the repository shown, with as many pages as its timeline has loaded. With no
/// repository shown any more, the schedule is told the read stopped.
fn start(state: &mut State, started: StartedRead) -> Option<Effect> {
    let Some(folder) = state.shown_read().map(|read| read.folder.clone()) else {
        stopped(state);
        return None;
    };
    let request = super::timeline::read_request(state, state.timeline.pages);
    let id = JobId(state.next_job);
    state.next_job += 1;
    let kind = JobKind::ReadRepository { folder, request };
    let _new = state.jobs.insert(id, kind.clone());
    let _counted = state.reads.scheduled.insert(id, started);
    Some(Effect::Start(Job { id, kind }))
}

/// Keeps only the last wake of a batch of changes: each moves the same read.
fn dedupe_wakes(effects: Vec<Effect>) -> Vec<Effect> {
    let last_wake = effects.iter().rposition(|effect| matches!(effect, Effect::WakeAt(_)));
    effects
        .into_iter()
        .enumerate()
        .filter(|(index, effect)| !matches!(effect, Effect::WakeAt(_)) || Some(*index) == last_wake)
        .map(|(_, effect)| effect)
        .collect()
}
