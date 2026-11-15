//! `journal/last` and `journal/record`: the newest operation Undo would take back, for its label, and an operation the
//! extension ran in TypeScript, recorded in the journal of the repository's engine so that Undo finds the newest one
//! whichever of the two ran it.

use std::sync::Arc;

use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::contract::{Text, UndoRecord};
use protocol::journal::{
    JournalEntryJson, JournalLastParams, JournalLastResult, JournalRecordParams, JournalRecordResult, head_of,
};
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

/// The answer of `journal/record`: the number of the entry made, if one was.
pub fn record(session: &Arc<Session>, params: JournalRecordParams) -> Result<Value, ResponseError> {
    let invalid =
        |problem: protocol::feature::NotACommit| ResponseError::new(codes::INVALID_PARAMS, problem.to_string());
    let record = params.record.map(UndoRecord::try_from).transpose().map_err(invalid)?;
    let before = head_of(&params.before).map_err(invalid)?;
    let after = head_of(&params.after).map_err(invalid)?;
    let title = Text::from(params.title);
    let repository = session.opened(&params.repository)?;
    let number = repository
        .with_engine(move |engine| {
            let before_newest = engine.journal().newest().map(|entry| entry.number);
            engine.record(&title, record.as_ref(), &before, &after);
            engine
                .journal()
                .newest()
                .map(|entry| entry.number)
                .filter(|number| Some(*number) != before_newest)
        })
        .map_err(|_busy| busy())?;
    json(&JournalRecordResult { number })
}

fn busy() -> ResponseError {
    ResponseError::new(codes::BUSY, "Another operation runs in this repository.")
}
