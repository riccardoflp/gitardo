//! `feature/run` and `question/validate`: a feature of the catalog carried to its end in a repository opened, its
//! questions asked of the extension, and the text of a question checked while it is typed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use engine::{EngineError, Finished, TaskSteps};
use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::contract::{Answer, AnswerKey, Answers, Context, Target, Validation};
use model::state::RepositoryState;
use protocol::feature::{
    AnswerJson, FeatureRunParams, FinishedJson, FollowUpJson, FollowUpsParams, ValidateParams, ValidateResult,
};
use protocol::messages::methods;
use protocol::rpc::{ErrorData, RequestId, ResponseError, codes, notification_message};
use serde_json::Value;

use crate::asker::HostAsker;
use crate::host::HostQuestions;
use crate::repository::Repository;
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
    let state = read_kept(&repository, cancel)?;
    let mut settings = session.feature_settings();
    settings.branch_values = params.inputs;
    let context = Context {
        repository: state,
        target,
        settings,
        now: now(),
    };
    let (known, given) = known_before(params.answers);
    let (asking, run, token) = (Arc::clone(session), id.clone(), cancel.clone());
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
            if let Some(host) = host_questions(&asking, &token, &context) {
                engine.ask_host_with(host);
            }
            let mut asker = HostAsker::new(&asking, run, given, token);
            let finished = engine.run_feature_from(feature, operation, (&context, known), &mut asker);
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
                session.read_after_plan(&params.repository, &repository);
            }
            json(&FinishedJson::from(&finished))
        }
        Err(error) => {
            if let EngineError::Step(_) = &error {
                session.read_after_plan(&params.repository, &repository);
            }
            Err(engine_failure(&error))
        }
    }
}

/// The state of a repository read for the context of a feature, and kept for `journal/last` (`undo.last`).
fn read_kept(repository: &Repository, cancel: &CancelToken) -> Result<Arc<RepositoryState>, ResponseError> {
    let folder = repository.folder().to_path_buf();
    let git = Git::new(&folder).with_cancel(cancel.clone());
    let started = repository.read_starts();
    let state = Arc::new(read_state(&git, &StateRequest { folder }).map_err(|error| git_failure(&error))?);
    repository.keep_state(started, Arc::clone(&state));
    Ok(state)
}

/// Area 6 of stage 4: the questions a feature asks of the extension in place of the credential store and the AI
/// provider, when it said it answers them: the token of GitHub and the language models are VS Code's.
fn host_questions(session: &Arc<Session>, cancel: &CancelToken, context: &Context) -> Option<Box<dyn engine::Host>> {
    let host = session.host();
    let ai = (*context.settings.ai).clone();
    let questions = HostQuestions::new(Arc::clone(session), cancel.clone(), None, ai);
    (host.github_token || host.language_model).then(|| Box::new(questions) as Box<dyn engine::Host>)
}

/// The answers given in `feature/run` split in two: the yes or no an operation offered was told (`flag`), and the
/// co-authors of the commit box (`commit.options`, 1.13), known from the first call as the window of the desktop knows
/// them, since no question asks them; and the answers of questions, which answer them when they are asked and are
/// checked then.
fn known_before(given: BTreeMap<String, AnswerJson>) -> (Answers, BTreeMap<String, AnswerJson>) {
    let mut known = Answers::new();
    let mut asked = BTreeMap::new();
    for (key, answer) in given {
        match answer {
            AnswerJson::Flag { value } => known.insert(AnswerKey(static_key(&key)), Answer::Flag(value)),
            AnswerJson::Choices { identifiers } if key == features::commit::CO_AUTHORS.0 => {
                known.insert(features::commit::CO_AUTHORS, Answer::Choices(identifiers));
            }
            answer => {
                let _replaced = asked.insert(key, answer);
            }
        }
    }
    (known, asked)
}

/// The key of an answer given, as the engine names its keys, which live as long as it does: each key is kept once, so
/// the few the features have are all that is ever kept.
fn static_key(key: &str) -> &'static str {
    static KEYS: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
    let mut keys = KEYS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(kept) = keys.get(key) {
        return kept;
    }
    let kept: &'static str = Box::leak(key.to_owned().into_boxed_str());
    let _new = keys.insert(kept);
    kept
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
