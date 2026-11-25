//! `journal/last`: the newest operation Undo would take back, for its label, checked against the state the last read
//! of the repository found.

use std::sync::Arc;

use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::state::RepositoryState;
use protocol::journal::{JournalEntryJson, JournalLastParams, JournalLastResult};
use protocol::rpc::{ResponseError, codes};
use serde_json::Value;

use crate::repository::Repository;
use crate::session::{Session, git_failure, json};

/// The answer of `journal/last`: the newest entry that still holds with the repository as its last read found it. That
/// state is the one the last read of the state kept (M15); the repository is read only when none was read since the
/// last operation.
pub fn last(session: &Arc<Session>, params: &JournalLastParams) -> Result<Value, ResponseError> {
    let repository = session.opened(&params.repository)?;
    let state = repository.kept_state().map_or_else(|| read_and_keep(&repository), Ok)?;
    let entry = repository
        .with_engine(move |engine| {
            engine.state_read(state);
            engine.journal().newest().map(JournalEntryJson::from)
        })
        .map_err(|_busy| busy())?;
    json(&JournalLastResult { entry })
}

/// Reads the state of a repository none was kept for, and keeps it.
fn read_and_keep(repository: &Repository) -> Result<Arc<RepositoryState>, ResponseError> {
    let started = repository.read_starts();
    let folder = repository.folder().to_path_buf();
    let read = read_state(&Git::new(&folder), &StateRequest { folder }).map_err(|error| git_failure(&error))?;
    let state = Arc::new(read);
    let _kept = repository.keep_state(started, Arc::clone(&state));
    Ok(state)
}

/// The error of a request while another operation runs in the repository.
fn busy() -> ResponseError {
    ResponseError::new(codes::BUSY, "Another operation runs in this repository.")
}
