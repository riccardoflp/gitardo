//! The picker of the co-authors of the next commit (`commit.co-authors`), as the store keeps it: the authors of the
//! history read on a worker when it opens, and that read stopped when it closes first (Q1096); the ticks kept as the
//! co-authors on `Enter`, and left as they were on `Escape`.

use std::path::PathBuf;

use git::read::commit::AuthorsRequest;
use model::commit::{Author, CoAuthor};
use views::changes::{CoAuthorsIntent, CoAuthorsState};

use super::message::{Effect, JobId, JobKind};
use super::state::State;

/// The picker while it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picking {
    /// The repository whose next commit it chooses for.
    pub folder: PathBuf,
    /// The read of the authors, while it runs.
    pub job: Option<JobId>,
    /// The authors read; none while they are read.
    pub authors: Option<Vec<Author>>,
    /// What is typed.
    pub typed: String,
    /// The co-authors ticked.
    pub ticked: Vec<CoAuthor>,
}

/// **Co-authors…**: opens the picker with the co-authors chosen ticked, and starts the read of the authors.
pub(super) fn open(state: &mut State) -> Vec<Effect> {
    let Some(folder) = state.shown_folder().cloned() else {
        return Vec::new();
    };
    let ticked = super::commit_options::in_force(state, &folder).co_authors;
    let kind = JobKind::ReadAuthors {
        folder: folder.clone(),
        request: AuthorsRequest,
    };
    let job = super::workspace::start_job(state, kind);
    state.changes.co_authors = Some(Picking {
        folder,
        job: Some(job.id),
        authors: None,
        typed: String::new(),
        ticked,
    });
    vec![Effect::Start(job)]
}

/// Something asked of the picker.
pub(super) fn reduce(state: &mut State, intent: CoAuthorsIntent) -> Vec<Effect> {
    let Some(picking) = state.changes.co_authors.as_mut() else {
        return Vec::new();
    };
    match intent {
        CoAuthorsIntent::Typed(text) => picking.typed = text,
        CoAuthorsIntent::Tick(person) => {
            let before = picking.ticked.len();
            picking
                .ticked
                .retain(|ticked| !ticked.email.eq_ignore_ascii_case(&person.email));
            if picking.ticked.len() == before {
                picking.ticked.push(person);
            }
        }
        CoAuthorsIntent::Keep => {
            let Some(picking) = state.changes.co_authors.take() else {
                return Vec::new();
            };
            let mut options = super::commit_options::in_force(state, &picking.folder);
            options.co_authors = picking.ticked;
            let _before = state.changes.options.insert(picking.folder, options);
            return picking.job.map(Effect::Cancel).into_iter().collect();
        }
        CoAuthorsIntent::Cancel => {
            let job = state.changes.co_authors.take().and_then(|picking| picking.job);
            return job.map(Effect::Cancel).into_iter().collect();
        }
    }
    Vec::new()
}

/// Takes the authors read for the picker that asked for them; a read no one waits for any more is dropped. A read that
/// failed lists no author, and what git said is the last error.
pub(super) fn authors_read(state: &mut State, job: JobId, result: Result<Vec<Author>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(picking) = state
        .changes
        .co_authors
        .as_mut()
        .filter(|picking| picking.job == Some(job))
    else {
        return Vec::new();
    };
    picking.job = None;
    match result {
        Ok(authors) => picking.authors = Some(authors),
        Err(error) => {
            picking.authors = Some(Vec::new());
            state.last_error = Some(error);
        }
    }
    Vec::new()
}

/// What the picker is drawn from, while it is open.
#[must_use]
pub fn picker_state(state: &State) -> Option<CoAuthorsState<'_>> {
    let picking = state.changes.co_authors.as_ref()?;
    let identity_email = state.shown_read().and_then(|read| read.state.identity.email.as_deref());
    Some(CoAuthorsState {
        authors: picking.authors.as_deref(),
        typed: &picking.typed,
        ticked: &picking.ticked,
        identity_email,
    })
}
