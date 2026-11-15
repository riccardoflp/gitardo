//! `feature/run` and `question/validate`: a feature of the catalog carried to its end in a repository opened, its
//! questions asked of the extension, and the text of a question checked while it is typed.

use std::sync::Arc;

use engine::{EngineError, Finished};
use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::contract::{Context, Target, Validation};
use protocol::feature::{
    FeatureRunParams, FinishedJson, FollowUpJson, FollowUpsParams, ValidateParams, ValidateResult,
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
    let (asking, run, given, token) = (Arc::clone(session), id.clone(), params.answers, cancel.clone());
    let finished = repository
        .with_engine(move |engine| {
            engine.state_read(Arc::clone(&context.repository));
            engine.cancel_with(token.clone());
            let mut asker = HostAsker::new(&asking, run, given, token);
            let finished = engine.run_feature(feature, operation, &context, &mut asker);
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
