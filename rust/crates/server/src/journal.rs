//! `journal/last`: the newest operation Undo would take back, for its label.

use std::sync::Arc;

use git::Git;
use git::read::repo::{StateRequest, read_state};
use protocol::journal::{JournalEntryJson, JournalLastParams, JournalLastResult};
use protocol::rpc::{ResponseError, codes};
use serde_json::Value;

use crate::session::{Session, git_failure, json};

/// The answer of `journal/last`: the newest entry that still holds with the repository as it is now.
pub fn last(session: &Arc<Session>, params: &JournalLastParams) -> Result<Value, ResponseError> {
    let repository = session.opened(&params.repository)?;
    let folder = repository.folder().to_path_buf();
    let state = read_state(&Git::new(&folder), &StateRequest { folder }).map_err(|error| git_failure(&error))?;
    let entry = repository
        .with_engine(move |engine| {
            engine.state_read(Arc::new(state));
            engine.journal().newest().map(JournalEntryJson::from)
        })
        .map_err(|_busy| busy())?;
    json(&JournalLastResult { entry })
}

fn busy() -> ResponseError {
    ResponseError::new(codes::BUSY, "Another operation runs in this repository.")
}
