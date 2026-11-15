//! `feature/run` and `question/validate`: a feature of the catalog carried to its end in a repository opened, its
//! questions asked of the extension, and the text of a question checked while it is typed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use engine::{EngineError, Finished, TaskSteps};
use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::contract::{Answer, AnswerKey, Answers, Context, Target, Validation};
use protocol::feature::{
    AnswerJson, FeatureRunParams, FinishedJson, FollowUpJson, FollowUpsParams, ValidateParams, ValidateResult,
};
use protocol::messages::methods;
use protocol::rpc::{ErrorData, RequestId, ResponseError, codes, notification_message};
use serde_json::Value;

use crate::asker::HostAsker;
use crate::session::{Session, git_failure, json, now};

/// Runs a feature for `feature/run` of this id, on the thread of the repository's engine: its answer, or the error of
/// what stopped it. A second one in the same repository while one runs is answered `busy`.
pub fn run(
    session: &Arc<Session>,
    id: &RequestId,
    params: FeatureRunParams,
    cancel: &CancelToken,
) -> Result<Value, ResponseError> {
    let Some((feature, operation)) = features::operation(&params.feature) else {
        let message = format!("Gitardo's engine has no feature named {}.", params.feature);
        return Err(ResponseError::new(codes::UNKNOWN_FEATURE, message));
    };
    let target = Target::try_from(params.target)
        .map_err(|problem| ResponseError::new(codes::INVALID_PARAMS, problem.to_string()))?;
    let repository = session.opened(&params.repository)?;
    let folder = repository.folder().to_path_buf();
    let git = Git::new(&folder).with_cancel(cancel.clone());
    let state = read_state(&git, &StateRequest { folder: folder.clone() }).map_err(|error| git_failure(&error))?;
    let mut settings = session.feature_settings();
    settings.branch_values = params.inputs;
    let context = Context {
        repository: Arc::new(state),
        target,
        settings,
        now: now(),
    };
    let read_unasked = confirmed_given(&params.answers);
    let (asking, run, given, token) = (Arc::clone(session), id.clone(), params.answers, cancel.clone());
    // The setup commands of a new worktree run in VS Code's terminals when the extension said it runs them.
    let tasks = if session.runs_tasks() {
        TaskSteps::HandedOver
    } else {
        TaskSteps::InPlace
    };
    let finished = repository
        .with_engine(move |engine| {
            engine.state_read(Arc::clone(&context.repository));
            engine.cancel_with(token.clone());
            engine.run_tasks(tasks);
            let mut asker = HostAsker::new(&asking, run, given, token);
            let finished = engine.run_feature_from(feature, operation, (&context, read_unasked), &mut asker);
            (finished, engine.take_log(asking.log_level()))
        })
        .map_err(|_busy| ResponseError::new(codes::BUSY, "Another operation runs in this repository."))?;
    let (finished, log) = finished;
    session.send_log(&params.repository, log);
    match finished {
        Ok(Finished::BackedOut) if cancel.is_cancelled() => Err(cancelled()),
        Ok(finished) => {
            if let Finished::Ran { plan, .. } = &finished {
                let follow_ups = FollowUpsParams {
                    run: id.clone(),
                    follow_ups: FollowUpJson::all_of(&plan.after),
                };
                session.send(notification_message(methods::FEATURE_FOLLOW_UPS, json(&follow_ups)?));
                session.read_after_plan(&params.repository, &folder);
            }
            json(&FinishedJson::from(&finished))
        }
        Err(error) => {
            if let EngineError::Step(_) = &error {
                session.read_after_plan(&params.repository, &folder);
            }
            Err(engine_failure(&error))
        }
    }
}

/// The answers given that a feature reads without asking, as **Remove anyway** reads `force`: those that confirm,
/// which have nothing to check. The others answer the questions of their keys when they are asked.
fn confirmed_given(given: &BTreeMap<String, AnswerJson>) -> Answers {
    given
        .iter()
        .filter(|(_, answer)| matches!(answer, AnswerJson::Confirmed))
        .fold(Answers::new(), |answers, (name, _)| {
            answers.with(key_named(name), Answer::Confirmed)
        })
}

/// The key of an answer by its name. The keys of the engine are names written in its code, so each name the extension
/// sends is kept once for the life of the engine, however often it comes back.
fn key_named(name: &str) -> AnswerKey {
    static NAMES: OnceLock<Mutex<BTreeSet<&'static str>>> = OnceLock::new();
    let mut names = NAMES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(&known) = names.get(name) {
        return AnswerKey(known);
    }
    let kept: &'static str = Box::leak(name.to_owned().into_boxed_str());
    let _new = names.insert(kept);
    AnswerKey(kept)
}

/// The answer of `question/validate`: what is wrong with the text, or nothing.
pub fn validate(params: ValidateParams) -> Result<Value, ResponseError> {
    let problem = Validation::from(params.validation).problem(&params.text);
    json(&ValidateResult {
        problem: problem.map(|text| text.to_string()),
    })
}

/// The error of a request cancelled with `$/cancel`.
fn cancelled() -> ResponseError {
    ResponseError::new(codes::CANCELLED, "The operation was cancelled.")
}

/// The error of a feature that did not end: a read that failed, a step that failed, an answer refused, or a feature
/// that never decides.
fn engine_failure(error: &EngineError) -> ResponseError {
    match error {
        EngineError::Query(failed) => git_failure(failed),
        EngineError::Step(failure) if failure.cancelled => cancelled(),
        EngineError::Step(failure) => {
            let data = ErrorData {
                output: Some(failure.said.clone()).filter(|said| !said.is_empty()),
                steps: failure.ran.clone(),
                follow_ups: FollowUpJson::all_of(&failure.on_failure),
            };
            let error = ResponseError::new(codes::STEP_FAILED, error.to_string());
            match serde_json::to_value(&data) {
                Ok(data) => error.with_data(data),
                Err(_) => error,
            }
        }
        EngineError::AnswerRefused { .. } => ResponseError::new(codes::ANSWER_REFUSED, error.to_string()),
        EngineError::EndlessFeature(_) => ResponseError::new(codes::ENDLESS_FEATURE, error.to_string()),
    }
}
