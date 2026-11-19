//! The operations run from the window (`docs/desktop/02-architecture.md`, From a click to git): the one running, the
//! question it asks or the plan it shows first (`preview.confirm`), and what is said once it ended. The engine runs it
//! on a thread of its own; the store only keeps what the window shows of it, and says what the user answered.

use std::path::PathBuf;
use std::sync::Arc;

use engine::LogEntry;
use model::CommitId;
use model::contract::{Answer, Answers, Context, FeatureId, FollowUp, Plan, Question, QuestionKind, Target, Text};
use model::repo::ReadCause;
use model::state::RepositoryState;
use views::feature::FeatureIntent;
use views::text::feature as words;

use super::{Effect, State, reads, search};
use crate::persisted::inputs::{inputs_of, remember_inputs};

mod after_read;
mod said;
mod shown;

pub use after_read::after_read;
pub use said::Said;
pub use shown::{plan_shown, question_state, said_state};

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
        /// The steps handed over from the first task on, which what follows waits for (`worktree-setup.command-steps`).
        waiting: Option<Box<super::tasks_tab::Waiting>>,
    },
    /// It refused to run, and said why, with what it offers to do instead.
    Refused(Text, Option<(FeatureId, Text)>),
    /// The user backed out of a question, or did not run the plan shown: nothing ran.
    BackedOut,
    /// A step failed, or a read it needed: what it was, and what git or the system said.
    Failed {
        /// What was being done: the title of the plan, or the operation.
        title: String,
        /// What was said.
        said: String,
        /// The title of the plan whose step failed; empty when no plan ran.
        plan: String,
        /// What the plan has the window do when it fails: the operations offered for what was said.
        on_failure: Vec<FollowUp>,
    },
}

/// The operation running, and what it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Running {
    /// Which operation.
    pub feature: FeatureId,
    /// The question it asks, or the plan it shows; none while it reads or runs.
    pub asking: Option<Asking>,
    /// The title of its plan, once it showed one.
    pub title: Option<String>,
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
    /// Whether to tell, once the repository is read again after an answer to a bisect, the first bad commit it found.
    pub check_bisect: bool,
    /// The title of the plan that failed, to tell once the repository is read again whether it stopped on conflicts.
    pub check_stopped: Option<String>,
    /// An operation on a file of another repository of the workspace, run once the window shows that repository
    /// (`compare.path-with`): the folder of the repository, the operation and its target.
    pub run_after_read: Option<(PathBuf, FeatureId, Target)>,
}

/// What is asked of an operation, or answered to it.
pub fn reduce(state: &mut State, intent: FeatureIntent) -> Vec<Effect> {
    match intent {
        // The two sides of a comparison are chosen together in the comparison chooser (`compare.chooser`), which runs
        // the operation with both answered.
        FeatureIntent::Run(FeatureId("compare.choose"), _) => super::compare::chooser::open(state),
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
        FeatureIntent::SaidShown(at) => {
            if let Some(said) = state.operations.said.as_mut() {
                said.shown_at = Some(at);
            }
            Vec::new()
        }
    }
}

/// Starts an operation on what it was invoked on, with the answers it was given already, in the repository shown,
/// unless one runs already. **Compare branches, tags or commits…** is given the comparisons opened before; **Open
/// settings**, offered by an operation switched off, opens the settings page.
fn run(state: &mut State, feature: FeatureId, target: Target, mut preset: Answers) -> Vec<Effect> {
    if feature == FeatureId("settings.edit") {
        super::pages::show_page(state, views::page::Page::Settings);
        return Vec::new();
    }
    if feature == FeatureId("worktree-setup.page") {
        return super::new_worktrees::open(state);
    }
    if let Some(effects) = super::draft::copilot_button(state, feature) {
        return effects;
    }
    if let Some(effects) = switch_first(state, feature, &target) {
        return effects;
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let repository = (read.folder.clone(), Arc::clone(&read.state));
    if feature == FeatureId("compare.choose") {
        let recent = super::compare::recent_of(state, &repository.0);
        preset.insert(features::compare::RECENT, Answer::Recent(recent));
    }
    run_in(state, repository, (feature, target), preset)
}

/// The operations on a file that show the repository holding it first, when it is another repository of the workspace:
/// its history (`repo.owner-of-file`) and its comparison with another (`compare.path-with`).
const SHOW_HOLDING_FIRST: [FeatureId; 2] = [FeatureId("compare.path-with"), FeatureId("file-history.file")];

/// Points the window at the repository of a file or a folder of another repository of the workspace, for an operation
/// that runs on it, which then runs once that repository is read: at once when its tab is up to date; none when the
/// file is in the repository shown, or in none of the workspace.
fn switch_first(state: &mut State, feature: FeatureId, target: &Target) -> Option<Vec<Effect>> {
    let Target::Path(path) = target else {
        return None;
    };
    if !SHOW_HOLDING_FIRST.contains(&feature) || state.operations.running.is_some() {
        return None;
    }
    let shown = state.shown_folder()?;
    if model::repo::path_in_folder(path, shown).is_some() {
        return None;
    }
    let holding = super::workspace::repository_holding(state, path)?;
    let mut effects = super::tabs::show_repository(state, &holding);
    let is_shown = state
        .shown_folder()
        .is_some_and(|shown| crate::persisted::same_folder(shown, &holding));
    if !is_shown {
        return (!effects.is_empty()).then_some(effects);
    }
    let is_read = state
        .shown_read()
        .is_some_and(|read| crate::persisted::same_folder(&read.folder, &holding));
    if is_read && !state.reads.schedule.is_reading() {
        effects.extend(run(state, feature, target.clone(), Answers::new()));
    } else {
        state.operations.run_after_read = Some((holding, feature, target.clone()));
    }
    Some(effects)
}

/// Starts an operation in a repository as it was last read, unless one runs already.
pub fn run_in(
    state: &mut State,
    (folder, repository): (PathBuf, Arc<RepositoryState>),
    (feature, target): (FeatureId, Target),
    preset: Answers,
) -> Vec<Effect> {
    if state.operations.running.is_some() {
        return super::tabs::refuse_operation(state);
    }
    let mut settings = state.settings.features.clone();
    settings.user_settings_file.clone_from(&state.user_settings_file);
    if let Some(workspace) = &state.workspace {
        settings.branch_values = inputs_of(&state.persisted.branch_inputs, &workspace.folders);
    }
    super::worktrees::settings_of(state, &mut settings.worktrees);
    super::trust::settings_of(state, &folder, &mut settings);
    let context = Context {
        repository,
        target,
        settings,
        // The store reads no clock: the window sets the time when it starts the operation.
        now: model::Timestamp(0),
    };
    state.operations.running = Some(Running {
        feature,
        asking: None,
        title: None,
    });
    state.operations.said = None;
    state.is_operation_running = true;
    super::tabs::operation_started(state);
    vec![Effect::RunFeature {
        folder,
        feature,
        preset,
        context: Box::new(context),
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

/// The operation running asks a question: in a dialog of the window, or, for a file to open or where to save one, in
/// the dialog of the system.
pub fn asks(state: &mut State, question: Question) -> Vec<Effect> {
    if let QuestionKind::OpenFile(file) | QuestionKind::SaveFile(file) = &question.kind {
        return vec![Effect::ChooseFile {
            title: question.title.as_str().to_owned(),
            question: file.clone(),
            is_saving: matches!(question.kind, QuestionKind::SaveFile(_)),
        }];
    }
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
    Vec::new()
}

/// The operation running shows its plan before it runs.
pub fn confirms(state: &mut State, plan: Plan) -> Vec<Effect> {
    if let Some(running) = state.operations.running.as_mut() {
        running.title = Some(plan.title.as_str().to_owned());
        running.asking = Some(Asking::Plan(plan));
    }
    Vec::new()
}

/// The operation running ended: its lines go to the Command log, what it says is said, with the line of the command
/// that said it, and the repository is read again when it changed something.
pub fn ended(state: &mut State, ended: Ended, log: Vec<LogEntry>) -> Vec<Effect> {
    let feature = state.operations.running.take().map(|running| running.feature);
    state.is_operation_running = false;
    let log_at = said::append_log(state, log);
    super::tasks_tab::operation_ended(state, feature);
    let mut effects = super::review::ended(state, feature, matches!(ended, Ended::Ran { .. }));
    effects.extend(super::trust::keep(state, &ended));
    match super::tasks_tab::hold(state, ended) {
        Ok(held) => effects.extend(held),
        Err(ended) => {
            effects.extend(ended_as_said(state, (*ended, feature)));
            if let Some(said) = state.operations.said.as_mut() {
                said.log_at = log_at;
            }
        }
    }
    effects
}

/// What an operation that ended says, and what it asks next.
pub(super) fn ended_as_said(state: &mut State, (ended, feature): (Ended, Option<FeatureId>)) -> Vec<Effect> {
    match ended {
        Ended::Ran {
            title, after, can_undo, ..
        } => {
            let said = remember_follow_ups(state, &after);
            let said_nothing = said.is_none();
            let is_undo = feature == Some(FeatureId("undo.last"));
            let pushed = feature.and_then(|feature| super::github::offer_after_push(state, feature));
            state.operations.said = Some(Said {
                can_undo: can_undo && !is_undo,
                offer: offer_of(&after),
                runs: super::worktrees::offered(&after),
                link: pushed
                    .as_ref()
                    .map(|notice| (notice.button.as_str().to_owned(), notice.url.clone())),
                ..Said::new(
                    pushed.map_or_else(
                        || said.unwrap_or_else(|| words::done(title.as_str())),
                        |notice| notice.message.as_str().to_owned(),
                    ),
                    false,
                )
            });
            // An operation that only runs or stops tasks says nothing of its own: the Tasks tab shows them.
            if said_nothing && after.iter().any(super::tasks_tab::is_follow_up) {
                state.operations.said = None;
            }
            let mut effects = reads::asked(state, ReadCause::OperationEnded);
            effects.extend(super::github::after_operation(state, &after));
            effects.extend(carry_out(state, after));
            state.compares.changing = None;
            effects
        }
        Ended::Refused(reason, offer) => {
            state.compares.changing = None;
            state.operations.said = Some(Said {
                offer: offer.map(|(feature, label)| (feature, label.as_str().to_owned())),
                ..Said::new(reason.as_str().to_owned(), true)
            });
            Vec::new()
        }
        Ended::BackedOut => {
            state.compares.changing = None;
            Vec::new()
        }
        Ended::Failed {
            title,
            said,
            plan,
            on_failure,
        } => {
            state.operations.said = Some(Said {
                runs: super::worktrees::offered(&on_failure),
                ..Said::new(words::failed(&title, &said), true)
            });
            state.operations.check_stopped = Some(plan).filter(|plan| !plan.is_empty());
            let mut effects = reads::asked(state, ReadCause::OperationEnded);
            effects.extend(carry_out(state, on_failure));
            effects
        }
    }
}

/// What the last offer among the follow-ups of an operation that ran offers, with the words of its button.
fn offer_of(after: &[FollowUp]) -> Option<(FeatureId, String)> {
    after.iter().rev().find_map(|follow_up| match follow_up {
        FollowUp::Offer(feature, label) => Some((*feature, label.as_str().to_owned())),
        _ => None,
    })
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
            FollowUp::CheckBisect => state.operations.check_bisect = true,
            FollowUp::CheckAutostash => {
                state.operations.autostash_before = state.shown_read().map(|read| read.state.stashes.clone());
            }
            _ => {}
        }
    }
    said
}

/// Keeps the values given for the inputs of `branches.format` as the ones used last in the workspace of the window
/// (`branch.name-from-format`).
fn remember_values(state: &mut State, values: &std::collections::BTreeMap<String, String>) -> Option<Effect> {
    let folders = state.workspace.as_ref()?.folders.clone();
    remember_inputs(&mut state.persisted.branch_inputs, &folders, values);
    Some(Effect::WritePersisted(state.persisted.clone()))
}

/// What an operation that ran asks of the window after it: a page opened, text on the clipboard, a comparison shown.
fn carry_out(state: &mut State, after: Vec<FollowUp>) -> Vec<Effect> {
    let mut effects = Vec::new();
    for follow_up in after {
        match (follow_up, state.shown_folder().cloned()) {
            (FollowUp::OpenCommitPage(commit), Some(folder)) => {
                effects.extend(super::commit_page::open(state, folder, commit));
            }
            (FollowUp::CopyToClipboard(text), _) => effects.push(Effect::CopyText(text)),
            (FollowUp::OpenUrl(url), _) => effects.push(Effect::OpenUrl(url)),
            (FollowUp::RememberInputs(values), _) => effects.extend(remember_values(state, &values)),
            (FollowUp::OpenWindow(folder), _) => effects.push(Effect::OpenWindow(folder)),
            (FollowUp::OpenComparison(target), _) => effects.extend(super::compare::opened(state, *target)),
            (FollowUp::OpenFileDiff(sides), _) => {
                effects.extend(super::diff_page::open(state, views::diff::file_sides(&sides)));
            }
            (FollowUp::CloseMergePage(path), _) => super::merge_page::close_file(state, &path),
            (FollowUp::OpenReview(files), _) => {
                state.operations.said = None;
                effects.extend(super::review::open(state, Some(files)));
            }
            (FollowUp::CompareInPlace(sides), Some(folder)) => {
                effects.extend(super::compare::in_place::show(state, folder, *sides));
            }
            (FollowUp::ShowHistory(history), _) => {
                effects.extend(search::show_history(state, history.map(|history| *history)));
            }
            (FollowUp::SelectCommit(_), _) => effects.extend(super::pages::show_timeline(state)),
            (FollowUp::EditFile(path, line), _) => {
                let target = views::file_view::FileTarget {
                    path,
                    revision: None,
                    line: Some(line),
                };
                effects.extend(super::file_page::open(state, target));
            }
            (blamed @ (FollowUp::ShowBlame(_) | FollowUp::HideBlame(..) | FollowUp::OpenFile(_)), _) => {
                effects.extend(super::blame::carry_out(state, blamed));
            }
            (FollowUp::TagsOnRemote(changed), Some(folder)) => super::tag_marks::follow(state, &folder, &changed),
            (tasks @ (FollowUp::RunTasks(_) | FollowUp::StopTasks(_)), _) => {
                effects.extend(super::tasks_tab::carry_out(state, tasks));
            }
            (other, _) => effects.extend(super::worktrees::carry_out(state, other)),
        }
    }
    effects
}
