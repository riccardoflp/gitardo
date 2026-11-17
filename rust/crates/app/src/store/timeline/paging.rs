//! The pages of the timeline (`graph.load-more`): those of a read taken as git prints them, under the pages before
//! them; past the 100,000 commits of one read, the next ones asked for once for each number of commits loaded, read on
//! a worker that sends only the rows after those the window has; the reads cancelled when they are not wanted.

use std::sync::Arc;

use git::read::graph::{CommitPage, Parents};
use model::graph::Row;
use views::graph::TimelineSwitch;

use super::super::message::{Effect, Job, JobId, JobKind, Known, MorePages, TimelineRequest};
use super::super::state::State;
use super::{Change, Stream, commits_request, folds, put_page, relayout, relayout_added};

/// How the timeline of the repository shown is read now: the whole history as git prints it, a page at a time.
#[must_use]
pub fn read_request(state: &State) -> TimelineRequest {
    let switches = state.settings.switches;
    TimelineRequest {
        page_size: state.settings.page_size,
        pages: 1,
        parents: if switches.is_on(TimelineSwitch::FirstParent) {
            Parents::First
        } else {
            Parents::All
        },
        shows_stashes: switches.is_on(TimelineSwitch::ShowStashes) && !super::super::search::is_narrowed(state),
        shown: state.timeline.shown.clone(),
        narrowing: super::super::search::narrowing(state),
        alone: super::super::search::alone(state),
    }
}

/// Asks for the commits after the last one read, once for each number of commits loaded, while there are more and
/// nothing reads the timeline: past the 100,000 commits of one read.
pub fn load_more(state: &mut State, loaded: usize) -> Vec<Effect> {
    let timeline = &state.timeline;
    if timeline.asked_at == Some(loaded) || timeline.loaded() != loaded {
        return Vec::new();
    }
    read_after(state)
}

/// Reads the commits after the last one read, for a commit gone to further back than they are
/// (`graph.show-in-graph`); nothing when there are no more, or the timeline is being read.
pub fn load_up_to(state: &mut State, commits: usize) -> Vec<Effect> {
    if commits < state.timeline.loaded() {
        return Vec::new();
    }
    read_after(state)
}

/// Starts the read of the commits after the last one read, when there are more and nothing reads the timeline.
fn read_after(state: &mut State) -> Vec<Effect> {
    let timeline = &state.timeline;
    let is_wanted = timeline.has_more && !is_reading(state);
    let Some(read) = state.shown_read().filter(|_| is_wanted) else {
        return Vec::new();
    };
    let request = read_request(state);
    let kind = JobKind::ReadCommits {
        folder: read.state.folder.clone(),
        request: commits_request(&read.state, &request),
        known: known(timeline.read_rows()),
    };
    let job = start(state, kind);
    state.timeline.more_job = Some(job.id);
    state.timeline.asked_at = Some(state.timeline.loaded());
    vec![Effect::Start(job)]
}

/// Whether the timeline is being read: another history, the commits after the last one, or the pages after the first.
#[must_use]
pub fn is_reading(state: &State) -> bool {
    let timeline = &state.timeline;
    timeline.is_reading_again || timeline.more_job.is_some() || timeline.stream.is_some()
}

/// Takes the first page of the commits after the last one read: its rows are added under those loaded, so that the
/// scroll position, the selection and the folds stay; when the history changed under them, every row is replaced. The
/// pages after it follow as git prints them.
pub fn commits_read(state: &mut State, job: JobId, result: Result<MorePages, String>) -> Vec<Effect> {
    let Some(JobKind::ReadCommits { .. }) = state.jobs.remove(&job) else {
        return Vec::new();
    };
    if state.timeline.more_job != Some(job) {
        return Vec::new();
    }
    state.timeline.more_job = None;
    let (follows, change) = match result {
        Ok(MorePages::Appended {
            rows,
            has_more,
            follows,
        }) => {
            state.timeline.rows.extend(rows);
            state.timeline.has_more = has_more;
            (follows, Change::Added)
        }
        Ok(MorePages::Replaced(page)) => {
            let draft: Vec<Row> = state
                .timeline
                .rows
                .iter()
                .take_while(|row| row.id().is_none())
                .cloned()
                .collect();
            state.timeline.rows = draft.into_iter().chain(page.rows).collect();
            state.timeline.has_more = page.has_more;
            (page.follows, Change::Other)
        }
        Err(error) => {
            state.last_error = Some(error);
            return super::super::search::settle(state);
        }
    };
    let read_rows = state.timeline.read_rows().len();
    state.timeline.stream = follows.then(|| Stream::under(job, read_rows));
    pages_taken(state, change)
}

/// Takes a page of a read after its first, as git printed it: it goes under the pages before it, the rows listed
/// staying while they are the same, and the last page ends the read. The bar along the top runs only until the first
/// page; while the others come, the last row says more commits are loaded (`graph.load-more`).
pub fn pages_read(state: &mut State, job: JobId, result: Result<CommitPage, String>) -> Vec<Effect> {
    let Some(mut stream) = state.timeline.stream.take_if(|stream| stream.job == job) else {
        return Vec::new();
    };
    let page = match result {
        Ok(page) => page,
        Err(error) => {
            state.last_error = Some(error);
            return super::super::search::settle(state);
        }
    };
    let Some(repository) = state.shown_read().map(|read| Arc::clone(&read.state)) else {
        return Vec::new();
    };
    let change = put_page(state, &mut stream, page, &repository);
    pages_taken(state, change)
}

/// After a page is taken: the merges it brings folded as all are, the timeline laid out again when its rows changed (the
/// rows added under the others alone), a commit being gone to selected once it is there, and the folds of merges gone
/// forgotten once the whole history is.
fn pages_taken(state: &mut State, change: Change) -> Vec<Effect> {
    if change != Change::Nothing {
        folds::fold_new_merges(state);
    }
    match change {
        Change::Nothing => {}
        Change::Added => relayout_added(state),
        Change::Other => relayout(state),
    }
    let mut effects = super::super::search::settle(state);
    let timeline = &state.timeline;
    if !timeline.has_more && timeline.stream.is_none() {
        effects.extend(folds::forget_gone(state));
    }
    effects
}

/// Reads the timeline again from one page, for another history: the rows listed stay, dimmed, until the new ones are
/// there. The read of the next pages running is cancelled.
pub fn read_again(state: &mut State) -> Vec<Effect> {
    let mut effects = cancel_more(state);
    let Some(folder) = state.shown_read().map(|read| read.folder.clone()) else {
        return effects;
    };
    let reads: Vec<JobId> = state
        .jobs
        .iter()
        .filter(|(_, kind)| matches!(kind, JobKind::ReadRepository { .. }))
        .map(|(id, _)| *id)
        .collect();
    for id in reads {
        let _cancelled = state.jobs.remove(&id);
        effects.push(Effect::Cancel(id));
    }
    let request = read_request(state);
    let job = start(
        state,
        JobKind::ReadRepository {
            folder,
            request,
            kept: None,
        },
    );
    super::super::reads::adopt(state, job.id, model::repo::ReadCause::Filter);
    effects.push(Effect::Start(job));
    state.timeline.is_reading_again = true;
    effects
}

/// Cancels the read of the next pages, the pages still coming and the read of another history, when they run.
pub fn cancel_more(state: &mut State) -> Vec<Effect> {
    let mut effects = stop_stream(state);
    if let Some(job) = state.timeline.narrow_job.take() {
        let _cancelled = state.jobs.remove(&job);
        state.timeline.is_reading_again = false;
        effects.push(Effect::Cancel(job));
    }
    if let Some(job) = state.timeline.more_job.take() {
        let _cancelled = state.jobs.remove(&job);
        state.timeline.asked_at = None;
        effects.push(Effect::Cancel(job));
    }
    effects
}

/// Stops the read whose pages are still coming, as when another read of the timeline takes its place.
pub fn stop_stream(state: &mut State) -> Vec<Effect> {
    state
        .timeline
        .stream
        .take()
        .map(|stream| Effect::Cancel(stream.job))
        .into_iter()
        .collect()
}

/// Notes a job as started and gives it its number.
pub(super) fn start(state: &mut State, kind: JobKind) -> Job {
    let id = JobId(state.next_job);
    state.next_job += 1;
    let _new = state.jobs.insert(id, kind.clone());
    Job { id, kind }
}

/// What a window has of a timeline: how many rows, how many commits among them, and the last commit.
#[must_use]
pub fn known(rows: &[Row]) -> Known {
    let mut commits = rows.iter().filter(|row| matches!(row, Row::Commit(_)));
    Known {
        count: rows.len(),
        commits: commits.clone().count(),
        last: commits.next_back().and_then(Row::id).cloned(),
    }
}
