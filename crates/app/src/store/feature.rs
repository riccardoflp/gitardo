//! The operations run from the window (`docs/02-architecture.md`, From a click to git): the one running, the question
//! it asks or the plan it shows first (`preview.confirm`), and what is said once it ended. The engine runs it on a
//! thread of its own; the store only keeps what the window shows of it, and says what the user answered.

use std::sync::Arc;

use engine::LogEntry;
use model::CommitId;
use model::contract::{Answer, Answers, Context, FeatureId, FollowUp, Plan, Question, QuestionKind, Target, Text};
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
        /// The message git prepared for the next commit, read from the git folder when the plan asked for it
        /// (`FollowUp::PreparedMessage`).
        prepared_message: Option<String>,
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
    /// Whether it warns of a stash a pull left, with **Show Stashes** beside it (`stash.autostash-left`).
    pub offers_stashes: bool,
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
    /// How many commits before `HEAD`, following first parents, `Some(None)` selects instead of `HEAD` itself.
    pub select_before_head: usize,
    /// The stashes before a pull with `--autostash`, to tell once the repository is read again whether it left one
    /// (`stash.autostash-left`).
    pub autostash_before: Option<Vec<model::state::Stash>>,
}

/// What is asked of an operation, or answered to it.
pub fn reduce(state: &mut State, intent: FeatureIntent) -> Vec<Effect> {
    match intent {
        FeatureIntent::Run(feature, target) => run(state, feature, target, Answers::new()),
        FeatureIntent::RunWith(feature, target, preset) => run(state, feature, target, preset),
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

/// Starts an operation on what it was invoked on, with the answers it was given already, in the repository shown,
/// unless one runs already.
fn run(state: &mut State, feature: FeatureId, target: Target, preset: Answers) -> Vec<Effect> {
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
        preset,
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
        Ended::Ran {
            title, after, can_undo, ..
        } => {
            let said = remember_follow_ups(state, &after);
            let is_undo = feature == Some(FeatureId("undo.last"));
            state.operations.said = Some(Said {
                text: said.unwrap_or_else(|| words::done(title.as_str())),
                is_error: false,
                can_undo: can_undo && !is_undo,
                offers_stashes: false,
            });
            let mut effects = reads::asked(state, ReadCause::OperationEnded);
            for follow_up in after {
                match (follow_up, state.shown_folder().cloned()) {
                    (FollowUp::OpenCommitPage(commit), Some(folder)) => {
                        effects.extend(super::commit_page::open(state, folder, commit));
                    }
                    (FollowUp::CopyToClipboard(text), _) => effects.push(Effect::CopyText(text)),
                    _ => {}
                }
            }
            effects
        }
        Ended::Refused(reason) => {
            state.operations.said = Some(Said {
                text: reason.as_str().to_owned(),
                is_error: true,
                can_undo: false,
                offers_stashes: false,
            });
            Vec::new()
        }
        Ended::BackedOut => Vec::new(),
        Ended::Failed { title, said } => {
            state.operations.said = Some(Said {
                text: words::failed(&title, &said),
                is_error: true,
                can_undo: false,
                offers_stashes: false,
            });
            reads::asked(state, ReadCause::OperationEnded)
        }
    }
}

/// What the follow-ups of an operation that ran ask the window to remember until the repository is read again, and
/// what the last of them that speaks says.
fn remember_follow_ups(state: &mut State, after: &[FollowUp]) -> Option<String> {
    let mut said = None;
    for follow_up in after {
        match follow_up {
            FollowUp::Say(text) => said = Some(text.as_str().to_owned()),
            FollowUp::SelectHead => state.operations.select_after_read = Some(None),
            FollowUp::SelectBeforeHead(count) => {
                state.operations.select_after_read = Some(None);
                state.operations.select_before_head = *count;
            }
            FollowUp::SelectCommit(commit) => state.operations.select_after_read = Some(Some(commit.clone())),
            FollowUp::CheckAutostash => {
                state.operations.autostash_before = state.shown_read().map(|read| read.state.stashes.clone());
            }
            _ => {}
        }
    }
    said
}

/// Once the repository is read again after a pull with `--autostash`, warns when the changes it set aside were left in
/// a stash, with **Show Stashes** beside the warning.
fn autostash_left(state: &mut State) {
    let Some(before) = state.operations.autostash_before.take() else {
        return;
    };
    let notice = state.shown_read().and_then(|read| {
        model::stash::autostash_left(&model::stash::PullOutcome {
            had_autostash: true,
            succeeded: true,
            stashes_before: &before,
            stashes_after: &read.state.stashes,
        })
    });
    if let (Some(notice), Some(said)) = (notice, state.operations.said.as_mut()) {
        notice.message.as_str().clone_into(&mut said.text);
        said.is_error = false;
        said.offers_stashes = true;
    }
}

/// Once the repository is read again after an operation, the commit it said to select is gone to.
pub fn after_read(state: &mut State) -> Vec<Effect> {
    autostash_left(state);
    let Some(selected) = state.operations.select_after_read.take() else {
        return Vec::new();
    };
    let back = std::mem::take(&mut state.operations.select_before_head);
    let commit = selected.or_else(|| {
        let head = state.shown_read().and_then(|read| read.state.head.commit.clone())?;
        Some(before(&state.timeline.rows, head, back))
    });
    commit.map(|commit| search::go_to(state, commit)).unwrap_or_default()
}

/// The commit a number of commits before another, following first parents through the rows read; as far as they go.
fn before(rows: &[model::graph::Row], mut commit: CommitId, count: usize) -> CommitId {
    for _ in 0..count {
        let parent = rows
            .iter()
            .find(|row| row.id() == Some(&commit))
            .and_then(|row| row.parents().first().cloned());
        match parent {
            Some(parent) => commit = parent,
            None => break,
        }
    }
    commit
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
        offers_stashes: said.offers_stashes,
    })
}
