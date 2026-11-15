//! Carrying a feature from its first call to its end: its questions asked, its queries read, its plan shown when it
//! must be, then run.

use std::sync::Arc;
use std::time::Instant;

use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::contract::{
    Answer, Answers, Context, FeatureId, FollowUp, Operation, Outcome, Plan, Query, Question, Refusal, Step, Text, Undo,
};
use model::state::RepositoryState;

use crate::activity::Activities;
use crate::clock::Clock;
use crate::error::EngineError;
use crate::log::CommandLog;
use crate::preview::needs_confirmation;
use crate::queries;
use crate::secrets::{SecretStore, SystemSecrets};
use crate::steps::{StepFailure, StepRunner};
use crate::tasks::TaskSteps;
use crate::undo::{Journal, journal_after};

mod answers;
mod tokens;

use answers::check_answer;

/// How many times a feature may be called before the engine gives up on it: far more than any feature asks.
const MOST_CALLS: usize = 100;

/// How the engine finds the operation of an identifier, for the features a plan names to run when one of its steps
/// failed (`FollowUp::Recover`): the catalog's own lookup, which the engine cannot use itself.
pub type FindOperation = fn(&str) -> Option<(FeatureId, Operation)>;

/// Who answers the questions of a feature and confirms its plan: the dialogs of a window, or the arguments of the
/// command line.
pub trait Asker {
    /// The answer to a question, or none when the user backed out of it.
    fn ask(&mut self, question: &Question) -> Option<Answer>;
    /// Whether to run a plan the user is shown first: its title, its steps, and its danger.
    fn confirm(&mut self, plan: &Plan) -> bool;
    /// The text on the clipboard of the system, which only a window can read; none when it holds no text.
    fn clipboard(&mut self) -> Option<String> {
        None
    }
}

/// How a feature ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Finished {
    /// Its plan ran; the lines of its steps, as the log has them.
    Ran {
        /// The plan.
        plan: Plan,
        /// The lines of the steps that ran.
        steps: Vec<String>,
        /// The state of the repository read once it ran, when the journal needed it; none when it did not, or the read
        /// failed.
        state_after: Option<Arc<RepositoryState>>,
        /// The links and copies that failed while the steps after them went on, each with what the system said
        /// (`worktree-setup.link-and-copy-steps`).
        went_on_after: Vec<StepFailure>,
        /// The steps from the first task on, handed over to be run apart once the others ran
        /// (`worktree-setup.command-steps`); none when the tasks run in place.
        waiting: Vec<Step>,
    },
    /// It refused to run, and said why.
    Refused(Refusal),
    /// The user backed out of a question: nothing ran.
    BackedOut,
    /// The plan was shown and the user did not run it: nothing ran and nothing was logged.
    NotConfirmed(Plan),
}

/// The engine of one repository: it runs features and plans there, keeps their log and says what is running.
pub struct Engine {
    git: Git,
    log: CommandLog,
    clock: Box<dyn Clock>,
    activities: Activities,
    journal: Journal,
    last_state: Option<Arc<RepositoryState>>,
    cancel: CancelToken,
    find_operation: Option<FindOperation>,
    went_on_after: Vec<StepFailure>,
    secrets: Box<dyn SecretStore>,
    http: Option<http::Http>,
    task_steps: TaskSteps,
    host: Option<Box<dyn crate::Host>>,
}

impl Engine {
    /// An engine that runs git as `git` says, logs to `log` and tells the time from `clock`.
    #[must_use]
    pub fn new(git: Git, log: CommandLog, clock: Box<dyn Clock>) -> Self {
        Engine {
            git,
            log,
            clock,
            activities: Activities::new(),
            journal: Journal::new(),
            last_state: None,
            cancel: CancelToken::new(),
            find_operation: None,
            went_on_after: Vec::new(),
            secrets: Box::new(SystemSecrets),
            http: None,
            task_steps: TaskSteps::InPlace,
            host: None,
        }
    }

    /// Sends the requests of the features (the models of the AI provider) with this client: in a test, one that
    /// reaches only the server of the test. Without it, a client that trusts the certificates of the system is made
    /// when a request is first sent.
    pub fn send_requests_with(&mut self, http: http::Http) {
        self.http = Some(http);
    }

    /// Takes the lookup of the operations of the catalog, so that a plan that names a feature to run when one of its
    /// steps failed gets it run. Without it, a failed step is always the end.
    pub fn find_operations_with(&mut self, find_operation: FindOperation) {
        self.find_operation = Some(find_operation);
    }

    /// Says how the tasks of the plans are run: in place, or handed back with the steps after them in
    /// [`Finished::Ran`], for a [`crate::TaskRunner`] to run apart.
    pub fn run_tasks(&mut self, how: TaskSteps) {
        self.task_steps = how;
    }

    /// Takes the token that cancels what the engine runs from now on: the reads of the features and the steps of their
    /// plans. The worker that runs a job gives the job's token; cancelling it kills the process of the step running,
    /// and no step after it starts.
    pub fn cancel_with(&mut self, cancel: CancelToken) {
        self.git = self.git.clone().with_cancel(cancel.clone());
        self.cancel = cancel;
    }

    /// The journal of what Undo needs for the operations of Gitardo the reflog does not see.
    #[must_use]
    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// Takes in a state of the repository read for any reason, so that an entry of the journal that no longer holds,
    /// because something moved `HEAD` from a terminal, is dropped.
    pub fn state_read(&mut self, state: Arc<RepositoryState>) {
        if let Some(before) = &self.last_state {
            self.journal = journal_after(&self.journal, None, before, &state);
        }
        self.last_state = Some(state);
    }

    /// The log of what ran.
    #[must_use]
    pub fn log(&self) -> &CommandLog {
        &self.log
    }

    /// What is running.
    #[must_use]
    pub fn activities(&self) -> &Activities {
        &self.activities
    }

    /// Puts the plans this engine runs among the activities of a window, so that its activity indicator shows them.
    pub fn share_activities(&mut self, activities: Activities) {
        self.activities = activities;
    }

    /// Takes the lines the log recorded since the last time, as a window that keeps a log of its own does, and
    /// records from now on what `level` says.
    pub fn take_log(&mut self, level: crate::log::LogLevel) -> Vec<crate::log::LogEntry> {
        let entries = self.log.take();
        self.log.set_level(level);
        entries
    }

    /// Carries a feature to its end: asks, reads, refuses, or runs its plan, first showing it when its danger and
    /// `commandPreview` say so.
    ///
    /// # Errors
    ///
    /// When a query cannot be read, an answer is not accepted by its question, a step fails, or the feature never
    /// decides.
    pub fn run_feature(
        &mut self,
        feature: FeatureId,
        operation: Operation,
        context: &Context,
        asker: &mut dyn Asker,
    ) -> Result<Finished, EngineError> {
        self.run_feature_from(feature, operation, (context, Answers::new()), asker)
    }

    /// Carries a feature to its end as [`Engine::run_feature`] does, with answers it is given before it is first
    /// called: what the window knows that git does not, such as the comparisons opened before.
    ///
    /// # Errors
    ///
    /// As [`Engine::run_feature`].
    pub fn run_feature_from(
        &mut self,
        feature: FeatureId,
        operation: Operation,
        (context, mut answers): (&Context, Answers),
        asker: &mut dyn Asker,
    ) -> Result<Finished, EngineError> {
        for _call in 0..MOST_CALLS {
            match operation(context, &answers) {
                Outcome::Ask(question) => {
                    let Some(answer) = asker.ask(&question) else {
                        return Ok(Finished::BackedOut);
                    };
                    check_answer(&question, &answer)?;
                    answers.insert(question.key, answer);
                }
                Outcome::Read(key, Query::Journal) => {
                    answers.insert(key, Answer::Journal(self.journal_entry(context).cloned()));
                }
                Outcome::Read(key, Query::Clipboard) => answers.insert(key, Answer::Clipboard(asker.clipboard())),
                Outcome::Read(key, Query::GitHubToken { helper }) => {
                    let tokens::TokenFound::Found(token) = self.github_token(helper).map_err(EngineError::Query)?
                    else {
                        // The host asked the user to sign in, and they did not: nothing is asked in its place.
                        return Ok(Finished::BackedOut);
                    };
                    answers.insert(key, Answer::Token(token));
                }
                Outcome::Read(key, Query::AiModels { settings, key: typed }) => {
                    answers.insert(key, Answer::AiModels(self.ai_models(settings, typed)));
                }
                Outcome::Read(key, query) => {
                    let started = Instant::now();
                    let answer = queries::answer(&self.git, &query).map_err(EngineError::Query)?;
                    self.log
                        .record_read(self.clock.now(), queries::describe(&query), started.elapsed());
                    answers.insert(key, answer);
                }
                Outcome::Refuse(refusal) => return Ok(Finished::Refused(refusal)),
                Outcome::Run(plan) => return self.confirm_and_run(feature, plan, context, asker),
            }
        }
        Err(EngineError::EndlessFeature(feature))
    }

    fn confirm_and_run(
        &mut self,
        feature: FeatureId,
        plan: Plan,
        context: &Context,
        asker: &mut dyn Asker,
    ) -> Result<Finished, EngineError> {
        if needs_confirmation(&plan.danger, context.settings.command_preview) && !asker.confirm(&plan) {
            return Ok(Finished::NotConfirmed(plan));
        }
        let _running = self
            .activities
            .track(plan.title.clone(), Text::from(feature.0), self.clock.now());
        let (now, waiting) = crate::tasks::split(&plan.steps, self.task_steps);
        let waiting = waiting.to_vec();
        let ran = self.run_plan(&Plan {
            steps: now.to_vec(),
            ..plan.clone()
        });
        let went_on_after = std::mem::take(&mut self.went_on_after);
        self.clean_up(&plan);
        let steps = match ran {
            Ok(steps) => steps,
            Err(failure) => match self.recover(&plan, context, asker, failure) {
                Ok(steps) => steps,
                Err(mut failure) => {
                    failure.plan = plan.title.to_string();
                    failure.on_failure = for_the_window(&plan.on_failure, &failure);
                    if matches!(&plan.undo, Undo::Journal(record) if record.holds_after_a_stop()) {
                        let _state_after = self.after_plan(&plan, context);
                    }
                    self.delete_rebase_files(&plan.on_failure, context);
                    return Err(EngineError::Step(failure));
                }
            },
        };
        self.delete_rebase_files(&plan.after, context);
        self.keep_tokens(&plan.after);
        self.keep_ai_keys(&plan.after);
        let state_after = self.after_plan(&plan, context);
        Ok(Finished::Ran {
            plan,
            steps,
            state_after,
            went_on_after,
            waiting,
        })
    }

    /// Runs the features a plan names to get past a step that failed, in order, until one gets past it: the lines of
    /// all the steps that ran, the failed one included; or the failure to show, which is that of the feature that tried
    /// and failed, or the plan's own when none tried.
    fn recover(
        &mut self,
        plan: &Plan,
        context: &Context,
        asker: &mut dyn Asker,
        failure: crate::StepFailure,
    ) -> Result<Vec<String>, crate::StepFailure> {
        if let Some(offer) = offered_instead(plan, &failure) {
            if !asker.confirm(&offer) {
                return Err(failure);
            }
            let steps = self.run_plan(&offer)?;
            self.log.got_past_failure();
            let mut all = failure.ran;
            all.push(failure.failed);
            all.extend(steps);
            return Ok(all);
        }
        let Some(find_operation) = self.find_operation.filter(|_| !failure.cancelled) else {
            return Err(failure);
        };
        for follow_up in &plan.on_failure {
            let FollowUp::Recover(identifier) = follow_up else {
                continue;
            };
            let Some((feature, operation)) = find_operation(identifier.0) else {
                continue;
            };
            match self.run_feature(feature, operation, context, asker) {
                Ok(Finished::Ran { steps, .. }) => {
                    self.log.got_past_failure();
                    let mut all = failure.ran;
                    all.push(failure.failed);
                    all.extend(steps);
                    return Ok(all);
                }
                Err(EngineError::Step(later)) => return Err(later),
                // It found nothing to get past, or could not tell: the failure is the plan's own.
                Ok(Finished::Refused(_) | Finished::BackedOut | Finished::NotConfirmed(_)) | Err(_) => {}
            }
        }
        Err(failure)
    }

    /// Deletes the folders of the files of an interactive rebase that the follow-ups name, once no rebase is under way
    /// in the worktree: a rebase that stopped still reads them. The delete is logged as a step; one that fails leaves
    /// the folder, which the next rebase writes over.
    fn delete_rebase_files(&mut self, follow_ups: &[FollowUp], context: &Context) {
        if context.repository.git_folder.join("rebase-merge").exists() {
            return;
        }
        let steps: Vec<Step> = follow_ups
            .iter()
            .filter_map(|follow_up| match follow_up {
                FollowUp::DeleteWhenRebaseEnded(folder) if folder.exists() => {
                    Some(Step::Delete { path: folder.clone() })
                }
                _ => None, // Any other follow-up is for the window.
            })
            .collect();
        if !steps.is_empty() {
            // The rebase has run already, and its files are only left over: a failure to delete them is in the log.
            let _ = StepRunner {
                git: &self.git,
                log: &mut self.log,
                clock: self.clock.as_ref(),
                cancel: &self.cancel,
                went_on_after: Vec::new(),
            }
            .run(&steps);
        }
    }

    /// Runs the steps a plan says to run once its own ended, whether they succeeded or not, such as the temporary file
    /// of a patch deleted. Their failure is logged and goes no further: the plan ended already.
    fn clean_up(&mut self, plan: &Plan) {
        let cleanup: Vec<model::contract::Step> = plan
            .after
            .iter()
            .filter_map(|follow_up| match follow_up {
                model::contract::FollowUp::Cleanup(step) => Some(step.clone()),
                _ => None, // The other follow-ups are the window's.
            })
            .collect();
        for step in cleanup {
            // A cleanup that fails is in the log with what the system said; the plan itself ended already.
            let _cleaned = self.run_plan(&Plan {
                steps: vec![step],
                ..plan.clone()
            });
        }
    }

    /// The newest entry of the journal, when it still holds for the state the feature was given.
    fn journal_entry(&mut self, context: &Context) -> Option<&model::contract::JournalEntry> {
        self.state_read(Arc::clone(&context.repository));
        self.journal.newest()
    }

    /// Brings the journal up to date once a plan ran, reading the state again to see what it left.
    fn after_plan(&mut self, plan: &Plan, context: &Context) -> Option<Arc<RepositoryState>> {
        let request = StateRequest {
            folder: context.repository.folder.clone(),
        };
        let Ok(after) = read_state(&self.git, &request) else {
            // Without the state after, nothing can say the entries still hold: Undo goes by the reflog.
            self.journal = Journal::new();
            return None;
        };
        let after = Arc::new(after);
        self.journal = journal_after(&self.journal, Some(plan), &context.repository, &after);
        self.last_state = Some(Arc::clone(&after));
        Some(after)
    }

    /// Runs the steps of a plan in order, logging each before it starts, and stops at the first that fails, but for a
    /// link or a copy, which the steps after it go on from: those are kept for [`Engine::take_went_on_after`].
    ///
    /// # Errors
    ///
    /// The step that failed, what it said, and the steps that ran before it.
    pub fn run_plan(&mut self, plan: &Plan) -> Result<Vec<String>, StepFailure> {
        let mut runner = StepRunner {
            git: &self.git,
            log: &mut self.log,
            clock: self.clock.as_ref(),
            cancel: &self.cancel,
            went_on_after: Vec::new(),
        };
        let ran = runner.run(&plan.steps);
        let mut failed = runner.went_on_after;
        self.went_on_after.append(&mut failed);
        ran
    }

    /// Takes the links and copies that failed in the plans run since the last time, while the steps after them went on.
    pub fn take_went_on_after(&mut self) -> Vec<StepFailure> {
        std::mem::take(&mut self.went_on_after)
    }
}

/// The plan a plan whose step failed offers in its place for what git said, asked first: the plan again from that step
/// with the file protocol allowed (`submodule.allow-file-protocol`), or another plan for some words of git's
/// (`FollowUp::OfferWhenFailed`). None when the step was cancelled, or nothing is offered for what git said.
fn offered_instead(plan: &Plan, failure: &StepFailure) -> Option<Plan> {
    if failure.cancelled {
        return None;
    }
    plan.on_failure.iter().find_map(|follow_up| match follow_up {
        FollowUp::AllowFileProtocol => model::submodule::allow_file_protocol(&model::submodule::FailedPlan {
            plan,
            failed_step: failure.ran.len(),
            said: &failure.said,
        }),
        FollowUp::OfferWhenFailed(offer) if failure.said.contains(&offer.when) => Some(offer.plan.clone()),
        _ => None, // The other follow-ups get past nothing.
    })
}

/// The follow-ups of a plan that failed which the window carries out: the operations offered for what was said, those
/// with no words to look for whatever was said, and any other the window does. What the engine did itself, getting
/// past the stop or deleting the files of a rebase, is not among them.
fn for_the_window(on_failure: &[FollowUp], failure: &StepFailure) -> Box<[FollowUp]> {
    if failure.cancelled {
        return Box::default();
    }
    let said = failure.said.to_lowercase();
    on_failure
        .iter()
        .filter(|follow_up| match follow_up {
            FollowUp::Recover(_) | FollowUp::DeleteWhenRebaseEnded(_) | FollowUp::Cleanup(_) => false,
            FollowUp::OfferRun(offer) => {
                offer.when_said.is_empty() || offer.when_said.iter().any(|words| said.contains(&words.to_lowercase()))
            }
            _ => true, // The window carries out the others as it does after a plan.
        })
        .cloned()
        .collect()
}
