//! Area 6 of stage 4, AI and GitHub: the two reads that ask a language model of VS Code, the draft of a commit message
//! (`read/commit.draft-message`) and the question about one conflict (`read/ai.ask-conflict`). The engine reads what
//! it needs, writes the prompt, asks with `host/languageModel` and reads the answer; the extension shows the answer as
//! it comes, under the read it asked, and keeps the review of the conflicts. A read is logged with how long it took,
//! the time the model took to answer included. Nothing here changes the repository.

use std::sync::Arc;
use std::time::Instant;

use engine::ai::{AiError, ConflictRequest};
use git::read::changes::{WorkingTreeRequest, read_working_tree};
use git::read::commit::{DraftDiffRequest, read_draft_diff};
use git::read::review::{SideNamesRequest, read_side_names};
use git::{CancelToken, Git};
use model::ai::{AiSettings, QuestionInput};
use model::commit::DraftInput;
use model::review::ReviewConflict;
use protocol::ai::{AskConflictParams, ConflictAnswerJson, DraftJson, DraftParams, methods};
use protocol::messages::RepositoryParams;
use protocol::rpc::{RequestId, ResponseError, codes};
use serde_json::Value;

use crate::host::HostQuestions;
use crate::session::{Session, git_failure, json, parameters};

/// What the draft says when there is nothing to describe, as the commit box says it.
const NOTHING_TO_DESCRIBE: &str = "There are no changes to describe.";

/// What a read of the area is given.
struct Asking<'a> {
    /// The session, for the settings and the host questions.
    session: &'a Arc<Session>,
    /// The id of the read, which the extension shows the answer under.
    id: &'a RequestId,
    /// Git, run in the folder of the repository, with the token of the request.
    git: Git,
    /// The token of the request.
    cancel: &'a CancelToken,
}

impl Asking<'_> {
    /// The language model of the host, asked with the model and the effort of these settings.
    fn model(&self, ai: AiSettings) -> HostQuestions {
        HostQuestions::new(Arc::clone(self.session), self.cancel.clone(), Some(self.id.clone()), ai)
    }
}

/// Whether a method is one of the reads of this module.
pub fn is_ai_read(method: &str) -> bool {
    method == methods::DRAFT_MESSAGE || method == methods::ASK_CONFLICT
}

/// The answer of a read of the area in a repository opened, logged as a read with how long it took.
///
/// # Errors
///
/// When the repository is not open, the parameters are not the read's, git fails, or the read is cancelled.
pub fn answer(
    session: &Arc<Session>,
    id: &RequestId,
    method: &str,
    params: Value,
    cancel: &CancelToken,
) -> Result<Value, ResponseError> {
    let RepositoryParams { repository } = parameters(params.clone())?;
    let folder = session.opened(&repository)?.folder().to_path_buf();
    let asking = Asking {
        session,
        id,
        git: Git::new(&folder).with_cancel(cancel.clone()),
        cancel,
    };
    let started = Instant::now();
    let answer = if method == methods::DRAFT_MESSAGE {
        draft_message(&asking, &parameters(params)?)
    } else {
        ask_conflict(&asking, parameters(params)?)
    };
    session.log_read(&repository, format!("Read {method} in {repository}"), started.elapsed());
    answer
}

/// The draft of a commit message (`commit.draft-message`): the staged changes when something is staged, every change
/// and the untracked files otherwise, asked of VS Code's first model, as the extension asked it.
fn draft_message(asking: &Asking<'_>, params: &DraftParams) -> Result<Value, ResponseError> {
    let tree = read_working_tree(&asking.git, &WorkingTreeRequest).map_err(|error| git_failure(&error))?;
    if tree.staged.is_empty() && tree.unstaged.is_empty() && tree.untracked.is_empty() {
        return json(&DraftJson::Failed {
            message: String::from(NOTHING_TO_DESCRIBE),
        });
    }
    let is_staged = !tree.staged.is_empty();
    let diff = read_draft_diff(&asking.git, &DraftDiffRequest { is_staged }).map_err(|error| git_failure(&error))?;
    let new_files = if is_staged { Vec::new() } else { tree.untracked };
    let input = DraftInput {
        diff: &diff,
        new_files: &new_files,
        subjects: &params.subjects,
    };
    let mut model = asking.model(AiSettings::default());
    match engine::ai::draft(&mut model, &input, &mut |_| {}) {
        Ok(message) => json(&DraftJson::Drafted { message }),
        Err(AiError::Cancelled) => Err(cancelled()),
        Err(error) => json(&DraftJson::Failed {
            message: error.to_string(),
        }),
    }
}

/// The question about one conflict of the review (`ai.ask-conflict`), asked of the model of `ai.model` with the effort
/// of `ai.effort`, with the names of the two sides the engine reads itself.
fn ask_conflict(asking: &Asking<'_>, params: AskConflictParams) -> Result<Value, ResponseError> {
    let read = read_side_names(&asking.git, &SideNamesRequest).map_err(|error| git_failure(&error))?;
    let sides = model::conflicts::side_names(&read.input());
    let asked = params.conflict;
    let conflict = ReviewConflict {
        index: 0,
        fingerprint: String::new(),
        line: asked.start.saturating_add(1),
        span: asked.start..asked.end,
        current: asked.current,
        base: asked.base,
        incoming: asked.incoming,
        rule: None,
        stretch: None,
    };
    let lines: Vec<&str> = params.lines.iter().map(String::as_str).collect();
    let request = ConflictRequest::of(&QuestionInput {
        path: &params.path,
        sides: &sides,
        lines: &lines,
        conflict: &conflict,
    });
    let mut model = asking.model(*asking.session.feature_settings().ai);
    let answered = engine::ai::ask_conflict(&mut model, &request, &mut |_| {});
    if let Err(AiError::Cancelled) = answered {
        return Err(cancelled());
    }
    json(&ConflictAnswerJson::of(&answered))
}

/// The error of a read cancelled with `$/cancel`.
fn cancelled() -> ResponseError {
    ResponseError::new(codes::CANCELLED, "The read was cancelled.")
}
