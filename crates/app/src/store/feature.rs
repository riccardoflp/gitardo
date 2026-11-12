//! The operations run from the window (`docs/02-architecture.md`, From a click to git): the one running, the question
//! it asks or the plan it shows first (`preview.confirm`), and what is said once it ended. The engine runs it on a
//! thread of its own; the store only keeps what the window shows of it, and says what the user answered.

use std::sync::Arc;

use engine::LogEntry;
use model::CommitId;
use model::contract::{Answer, Context, FeatureId, FollowUp, Plan, Question, QuestionKind, Target, Text};
use model::repo::ReadCause;
use views::feature::{FeatureIntent, QuestionState, SaidState};
use views::text::feature as words;

use super::{Effect, State, reads, search};

/// What is answered to the operation running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The answer to the question shown; none when the user backed out of it.
    Answer(Option<Answer>),
    /// Whether the plan shown is to run.
    Confirm(bool),
}

/// How an operation ended, as the thread that ran it says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ended {
    /// Its plan ran.
    Ran {
        /// The title of the plan.
        title: Text,
        /// What follows it.
        after: Vec<FollowUp>,
        /// Whether Undo can take it back.
        can_undo: bool,
    },
    /// It refused to run, and said why.
    Refused(Text),
    /// The user backed out of a question, or did not run the plan shown: nothing ran.
    BackedOut,
    /// A step failed, or a read it needed: what it was, and what git or the system said.
    Failed {
        /// What was being done: the title of the plan, or the operation.
        title: String,
        /// What was said.
        said: String,
    },
}

/// The operation running, and what it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Running {
    /// Which operation.
    pub feature: FeatureId,
    /// The question it asks, or the plan it shows; none while it reads or runs.
    pub asking: Option<Asking>,
}

/// What the operation running shows over the window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// A question, as it is being answered.
    Question {
        /// The question.
        question: Question,
        /// What is typed.
        typed: String,
        /// The choices picked.
        picked: Vec<String>,
    },
    /// The plan, before it runs.
    Plan(Plan),
}

/// What was said once an operation ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// The sentence.
    pub text: String,
    /// Whether it says the operation failed or was refused.
    pub is_error: bool,
    /// Whether **Undo** is offered beside it.
    pub can_undo: bool,
}

/// The operations of a window.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Operations {
    /// The one running.
    pub running: Option<Running>,
    /// What was said once the last one ended, until it is put away.
    pub said: Option<Said>,
    /// The commit to select once the repository is read again after an operation; `Some(None)` selects `HEAD`.
    pub select_after_read: Option<Option<CommitId>>,
}

/// What is asked of an operation, or answered to it.
pub fn reduce(state: &mut State, intent: FeatureIntent) -> Vec<Effect> {
    match intent {
        FeatureIntent::Run(feature, target) => run(state, feature, target),
        FeatureIntent::Typed(text) => {
            if let Some(Asking::Question { typed, .. }) = asking(state) {
                *typed = text;
            }
            Vec::new()
        }
        FeatureIntent::Picked(choice) => {
            if let Some(Asking::Question { question, picked, .. }) = asking(state) {
                pick(question, picked, choice);
            }
            Vec::new()
        }
        FeatureIntent::Answer(answer) => reply(state, Reply::Answer(answer)),
        FeatureIntent::Confirm(is_confirmed) => reply(state, Reply::Confirm(is_confirmed)),
        FeatureIntent::Dismiss => {
            state.operations.said = None;
            Vec::new()
        }
    }
}

/// Starts an operation on what it was invoked on, in the repository shown, unless one runs already.
fn run(state: &mut State, feature: FeatureId, target: Target) -> Vec<Effect> {
    if state.operations.running.is_some() {
        return Vec::new();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let context = Context {
        repository: Arc::clone(&read.state),
        target,
        settings: state.settings.features.clone(),
        // The store reads no clock: the window sets the time when it starts the operation.
        now: model::Timestamp(0),
    };
    let folder = read.folder.clone();
    state.operations.running = Some(Running { feature, asking: None });
    state.operations.said = None;
    state.is_operation_running = true;
    vec![Effect::RunFeature {
        folder,
        feature,
        context,
    }]
}

/// What the operation running shows, when it shows something.
fn asking(state: &mut State) -> Option<&mut Asking> {
    state.operations.running.as_mut()?.asking.as_mut()
}

/// Picks a choice: the only one of a pick, or one more or one less of a question that takes many.
fn pick(question: &Question, picked: &mut Vec<String>, choice: String) {
    if matches!(question.kind, QuestionKind::PickMany { .. }) {
        if let Some(index) = picked.iter().position(|known| *known == choice) {
            let _unticked = picked.remove(index);
        } else {
            picked.push(choice);
        }
    } else {
        *picked = vec![choice];
    }
}

/// Answers the operation running, and puts away what it showed.
fn reply(state: &mut State, reply: Reply) -> Vec<Effect> {
    let Some(running) = state.operations.running.as_mut() else {
        return Vec::new();
    };
    running.asking = None;
    vec![Effect::ReplyToFeature(reply)]
}

/// The operation running asks a question.
pub fn asks(state: &mut State, question: Question) {
    let typed = match &question.default {
        Some(Answer::Text(text)) => text.clone(),
        _ => String::new(),
    };
    let picked = match &question.default {
        Some(Answer::Choice(choice)) => vec![choice.clone()],
        Some(Answer::Choices(choices)) => choices.clone(),
        _ => Vec::new(),
    };
    if let Some(running) = state.operations.running.as_mut() {
        running.asking = Some(Asking::Question {
            question,
            typed,
            picked,
        });
    }
}

/// The operation running shows its plan before it runs.
pub fn confirms(state: &mut State, plan: Plan) {
    if let Some(running) = state.operations.running.as_mut() {
        running.asking = Some(Asking::Plan(plan));
    }
}

/// The operation running ended: its lines go to the Command log, what it says is said, and the repository is read
/// again when it changed something.
pub fn ended(state: &mut State, ended: Ended, log: Vec<LogEntry>) -> Vec<Effect> {
    let feature = state.operations.running.take().map(|running| running.feature);
    state.is_operation_running = false;
    state.log.append(log);
    match ended {
        Ended::Ran { title, after, can_undo } => {
            let mut said = None;
            for follow_up in &after {
                match follow_up {
                    FollowUp::Say(text) => said = Some(text.as_str().to_owned()),
                    FollowUp::SelectHead => state.operations.select_after_read = Some(None),
                    FollowUp::SelectCommit(commit) => state.operations.select_after_read = Some(Some(commit.clone())),
                    _ => {}
                }
            }
            let is_undo = feature == Some(FeatureId("undo.last"));
            state.operations.said = Some(Said {
                text: said.unwrap_or_else(|| words::done(title.as_str())),
                is_error: false,
                can_undo: can_undo && !is_undo,
            });
            let mut effects = reads::asked(state, ReadCause::OperationEnded);
            for follow_up in after {
                if let (FollowUp::OpenCommitPage(commit), Some(folder)) = (follow_up, state.shown_folder().cloned()) {
                    effects.extend(super::commit_page::open(state, folder, commit));
                }
            }
            effects
        }
        Ended::Refused(reason) => {
            state.operations.said = Some(Said {
                text: reason.as_str().to_owned(),
                is_error: true,
                can_undo: false,
            });
            Vec::new()
        }
        Ended::BackedOut => Vec::new(),
        Ended::Failed { title, said } => {
            state.operations.said = Some(Said {
                text: words::failed(&title, &said),
                is_error: true,
                can_undo: false,
            });
            reads::asked(state, ReadCause::OperationEnded)
        }
    }
}

/// Once the repository is read again after an operation, the commit it said to select is gone to.
pub fn after_read(state: &mut State) -> Vec<Effect> {
    let Some(selected) = state.operations.select_after_read.take() else {
        return Vec::new();
    };
    let commit = selected.or_else(|| state.shown_read().and_then(|read| read.state.head.commit.clone()));
    commit.map(|commit| search::go_to(state, commit)).unwrap_or_default()
}

/// What the window shows of the question asked.
#[must_use]
pub fn question_state(state: &State) -> Option<QuestionState<'_>> {
    match state.operations.running.as_ref()?.asking.as_ref()? {
        Asking::Question {
            question,
            typed,
            picked,
        } => Some(QuestionState {
            question,
            typed,
            picked,
        }),
        Asking::Plan(_) => None,
    }
}

/// The plan shown before it runs.
#[must_use]
pub fn plan_shown(state: &State) -> Option<&Plan> {
    match state.operations.running.as_ref()?.asking.as_ref()? {
        Asking::Plan(plan) => Some(plan),
        Asking::Question { .. } => None,
    }
}

/// What was said once the last operation ended.
#[must_use]
pub fn said_state(state: &State) -> Option<SaidState<'_>> {
    state.operations.said.as_ref().map(|said| SaidState {
        text: &said.text,
        is_error: said.is_error,
        can_undo: said.can_undo,
    })
}
