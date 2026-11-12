//! The pages of the timeline (`graph.load-more`): the next one asked for once for each number of commits loaded, read
//! on a worker that sends only the rows after those the window has; the reads cancelled when they are not wanted.

use git::read::graph::Parents;
use model::graph::Row;
use views::graph::TimelineSwitch;

use super::super::message::{Effect, Job, JobId, JobKind, Known, MorePages, TimelineRequest};
use super::super::state::State;
use super::{commits_request, folds, relayout};

/// How the timeline of the repository shown is read now: as many pages as are loaded, at least one.
#[must_use]
pub fn read_request(state: &State, pages: usize) -> TimelineRequest {
    let switches = state.settings.switches;
    TimelineRequest {
        page_size: state.settings.page_size,
        pages: pages.max(1),
        parents: if switches.is_on(TimelineSwitch::FirstParent) {
            Parents::First
        } else {
            Parents::All
        },
        shows_stashes: switches.is_on(TimelineSwitch::ShowStashes) && !super::super::search::is_narrowed(state),
        shown: state.timeline.shown.clone(),
        narrowing: super::super::search::narrowing(state),
        alone: state.search.alone.clone(),
    }
}

/// Asks for the next page, once for each number of commits loaded, while there are more and none is being read.
pub fn load_more(state: &mut State, loaded: usize) -> Vec<Effect> {
    let timeline = &state.timeline;
    let is_wanted = timeline.has_more
        && timeline.more_job.is_none()
        && !timeline.is_reading_again
        && timeline.asked_at != Some(loaded)
        && timeline.loaded() == loaded;
    let Some(read) = state.shown_read().filter(|_| is_wanted) else {
        return Vec::new();
    };
    let request = read_request(state, timeline.pages + 1);
    let kind = JobKind::ReadCommits {
        folder: read.state.folder.clone(),
        request: commits_request(&read.state, &request),
        known: known(timeline.read_rows()),
    };
    let job = start(state, kind);
    state.timeline.more_job = Some(job.id);
    state.timeline.asked_at = Some(loaded);
    vec![Effect::Start(job)]
}

/// Loads the pages of the timeline up to a number, for a commit gone to further back than it is loaded
/// (`graph.show-in-graph`); nothing when as many are loaded or read, or there are no more.
pub fn load_up_to(state: &mut State, pages: usize) -> Vec<Effect> {
    let timeline = &state.timeline;
    let is_wanted = timeline.has_more && !timeline.is_reading_again && pages > timeline.pages;
    let Some(read) = state.shown_read().filter(|_| is_wanted) else {
        return Vec::new();
    };
    let request = read_request(state, pages);
    let kind = JobKind::ReadCommits {
        folder: read.state.folder.clone(),
        request: commits_request(&read.state, &request),
        known: known(timeline.read_rows()),
    };
    let mut effects = cancel_more(state);
    let job = start(state, kind);
    state.timeline.more_job = Some(job.id);
    state.timeline.asked_at = Some(state.timeline.loaded());
    effects.push(Effect::Start(job));
    effects
}

/// Takes the next pages read: the rows after those loaded are added under them, so that the scroll position, the
/// selection and the folds stay; when the history changed under them, every row is replaced.
pub fn commits_read(state: &mut State, job: JobId, result: Result<MorePages, String>) -> Vec<Effect> {
    let Some(JobKind::ReadCommits { request, .. }) = state.jobs.remove(&job) else {
        return Vec::new();
    };
    if state.timeline.more_job != Some(job) {
        return Vec::new();
    }
    state.timeline.more_job = None;
    match result {
        Ok(MorePages::Appended { rows, has_more }) => {
            state.timeline.rows.extend(rows);
            state.timeline.has_more = has_more;
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
        }
        Err(error) => {
            state.last_error = Some(error);
            return super::super::search::settle(state);
        }
    }
    state.timeline.pages = request.pages;
    folds::fold_new_merges(state);
    relayout(state);
    let mut effects = super::super::search::settle(state);
    if !state.timeline.has_more {
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
    let request = read_request(state, 1);
    let job = start(state, JobKind::ReadRepository { folder, request });
    super::super::reads::adopt(state, job.id, model::repo::ReadCause::Filter);
    effects.push(Effect::Start(job));
    state.timeline.is_reading_again = true;
    state.timeline.pages = 1;
    effects
}

/// Cancels the read of the next pages and the read of another history, when they run.
pub fn cancel_more(state: &mut State) -> Vec<Effect> {
    let mut effects = Vec::new();
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
