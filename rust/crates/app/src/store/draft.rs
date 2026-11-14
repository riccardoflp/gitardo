//! The message of the commit box drafted by the AI provider (`commit.draft-message`): asked when the sparkle is
//! pressed, never before, from the staged changes when something is staged and from every change otherwise; its answer
//! takes the place of the text of the box, which is left as it was when it is cancelled or fails.

use std::path::PathBuf;

use model::graph::Row;
use views::changes::ChangesIntent;

use super::feature::Said;
use super::{Effect, State};
use crate::persisted::changes::keep_message;
use crate::work::ai::{AiQuestion, AiReply, AiResult, AiWork, AskId, DraftJob};

/// How many subjects of the last commits are sent.
const SUBJECTS: usize = 10;

/// A message being drafted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drafting {
    /// The question.
    pub ask: AskId,
    /// The repository whose message box the answer goes in.
    pub folder: PathBuf,
}

/// The sparkle of the commit box, or the button that stops the draft.
pub fn reduce(state: &mut State, intent: &ChangesIntent) -> Vec<Effect> {
    match intent {
        ChangesIntent::DraftMessage => draft(state),
        ChangesIntent::StopDraft => match state.changes.drafting.take() {
            Some(drafting) => vec![Effect::StopAsking(vec![drafting.ask])],
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Asks the AI provider for the message of the changes of the repository shown; refuses with nothing changed, and
/// with no AI provider set up.
fn draft(state: &mut State) -> Vec<Effect> {
    if state.changes.drafting.is_some() {
        return Vec::new();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, tree) = (read.folder.clone(), &read.state.working_tree);
    if tree.staged.is_empty() && tree.unstaged.is_empty() && tree.untracked.is_empty() {
        state.operations.said = Some(Said::error(String::from(views::text::changes::NOTHING_TO_DESCRIBE)));
        return Vec::new();
    }
    let settings = (*state.settings.features.ai).clone();
    if settings.provider.is_none() {
        let mut said = Said::error(engine::ai::AiError::NotSetUp.to_string());
        said.offer = Some((
            model::contract::FeatureId("settings.edit"),
            String::from(views::text::review::OPEN_SETTINGS),
        ));
        state.operations.said = Some(said);
        return Vec::new();
    }
    let is_staged = !tree.staged.is_empty();
    let job = DraftJob {
        folder: folder.clone(),
        is_staged,
        new_files: if is_staged { Vec::new() } else { tree.untracked.clone() },
        subjects: subjects(state),
    };
    let ask = super::review::ai::next_ask(state);
    state.changes.drafting = Some(Drafting { ask, folder });
    vec![Effect::Ask(Box::new(AiQuestion {
        id: ask,
        settings,
        work: AiWork::Draft(job),
    }))]
}

/// The subjects of the last commits of the timeline, newest first, without the draft and the stashes.
fn subjects(state: &State) -> Vec<String> {
    state
        .timeline
        .rows
        .iter()
        .filter_map(|row| match row {
            Row::Commit(commit) => Some(commit.subject.clone()),
            Row::Stash { .. } | Row::Uncommitted { .. } => None,
        })
        .take(SUBJECTS)
        .collect()
}

/// What the question came to: the message drafted takes the place of the text of the box; a failure is said, and the
/// box is left as it was.
pub fn drafted(state: &mut State, ask: AskId, result: AiResult) -> Vec<Effect> {
    let Some(drafting) = state.changes.drafting.take_if(|drafting| drafting.ask == ask) else {
        return Vec::new();
    };
    match result {
        Ok(AiReply::Draft(message)) => {
            keep_message(&mut state.persisted.commit_messages, &drafting.folder, &message);
            vec![Effect::WritePersisted(state.persisted.clone())]
        }
        Ok(AiReply::Conflict(_)) => Vec::new(),
        Err(failed) => {
            state.operations.said = Some(Said::error(failed.text));
            Vec::new()
        }
    }
}
