//! Carrying a feature from its first call to its end: its questions asked, its queries read, its plan shown when it
//! must be, then run.

use std::time::Instant;

use git::Git;
use model::contract::{
    Answer, Answers, Context, FeatureId, Operation, Outcome, Plan, Question, QuestionKind, Refusal, Text,
};

use crate::activity::Activities;
use crate::clock::Clock;
use crate::error::EngineError;
use crate::log::CommandLog;
use crate::preview::needs_confirmation;
use crate::queries;
use crate::steps::{StepFailure, StepRunner};

/// How many times a feature may be called before the engine gives up on it: far more than any feature asks.
const MOST_CALLS: usize = 100;

/// Who answers the questions of a feature and confirms its plan: the dialogs of a window, or the arguments of the
/// command line.
pub trait Asker {
    /// The answer to a question, or none when the user backed out of it.
    fn ask(&mut self, question: &Question) -> Option<Answer>;
    /// Whether to run a plan the user is shown first: its title, its steps, and its danger.
    fn confirm(&mut self, plan: &Plan) -> bool;
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
        }
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
        let mut answers = Answers::new();
        for _call in 0..MOST_CALLS {
            match operation(context, &answers) {
                Outcome::Ask(question) => {
                    let Some(answer) = asker.ask(&question) else {
                        return Ok(Finished::BackedOut);
                    };
                    check_answer(&question, &answer)?;
                    answers.insert(question.key, answer);
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
        let steps = self.run_plan(&plan).map_err(EngineError::Step)?;
        Ok(Finished::Ran { plan, steps })
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
