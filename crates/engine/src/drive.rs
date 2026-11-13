//! Carrying a feature from its first call to its end: its questions asked, its queries read, its plan shown when it
//! must be, then run.

use std::sync::Arc;
use std::time::Instant;

use git::read::repo::{StateRequest, read_state};
use git::{CancelToken, Git};
use model::contract::{
    Answer, Answers, Context, FeatureId, FollowUp, Operation, Outcome, Plan, Query, Question, QuestionKind, Refusal,
    Step, Text, Undo,
};
use model::state::RepositoryState;

use crate::activity::Activities;
use crate::clock::Clock;
use crate::error::EngineError;
use crate::log::CommandLog;
use crate::preview::needs_confirmation;
use crate::queries;
use crate::steps::{StepFailure, StepRunner};
use crate::undo::{Journal, journal_after};

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
        }
    }

    /// Takes the lookup of the operations of the catalog, so that a plan that names a feature to run when one of its
    /// steps failed gets it run. Without it, a failed step is always the end.
    pub fn find_operations_with(&mut self, find_operation: FindOperation) {
        self.find_operation = Some(find_operation);
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
        let ran = self.run_plan(&plan);
        self.clean_up(&plan);
        let steps = match ran {
            Ok(steps) => steps,
            Err(failure) => match self.recover(&plan, context, asker, failure) {
                Ok(steps) => steps,
                Err(failure) => {
                    if matches!(&plan.undo, Undo::Journal(record) if record.holds_after_a_stop()) {
                        let _state_after = self.after_plan(&plan, context);
                    }
                    self.delete_rebase_files(&plan.on_failure, context);
                    return Err(EngineError::Step(failure));
                }
            },
        };
        self.delete_rebase_files(&plan.after, context);
        let state_after = self.after_plan(&plan, context);
        Ok(Finished::Ran {
            plan,
            steps,
            state_after,
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

    /// Brings the journal up to date once a plan ran, reading the state again when the journal has anything to keep or
    /// drop.
    fn after_plan(&mut self, plan: &Plan, context: &Context) -> Option<Arc<RepositoryState>> {
        if self.journal.is_empty() && !matches!(plan.undo, Undo::Journal(_)) {
            return None;
        }
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

    /// Runs the steps of a plan in order, logging each before it starts, and stops at the first that fails.
    ///
    /// # Errors
    ///
    /// The step that failed, what it said, and the steps that ran before it.
    pub fn run_plan(&mut self, plan: &Plan) -> Result<Vec<String>, StepFailure> {
        StepRunner {
            git: &self.git,
            log: &mut self.log,
            clock: self.clock.as_ref(),
            cancel: &self.cancel,
        }
        .run(&plan.steps)
    }
}

/// Refuses an answer its question would not accept, so that a caller with no dialog, such as the command line, cannot
/// give one.
fn check_answer(question: &Question, answer: &Answer) -> Result<(), EngineError> {
    let problem = match (&question.kind, answer) {
        (QuestionKind::Text { validation, .. }, Answer::Text(text)) => validation.problem(text),
        (QuestionKind::Pick { choices }, Answer::Choice(picked)) => {
            (!choices.iter().any(|choice| choice.identifier == *picked))
                .then(|| Text::from(format!("{picked} is not one of the choices")))
        }
        (QuestionKind::Confirm { .. }, Answer::Confirmed) | (QuestionKind::PickMany { .. }, Answer::Choices(_)) => None,
        _ => Some(Text::from("This is not the kind of answer the question asks for")),
    };
    match problem {
        Some(problem) => Err(EngineError::AnswerRefused {
            question: question.title.clone(),
            problem,
        }),
        None => Ok(()),
    }
}
