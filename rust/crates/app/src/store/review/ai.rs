//! The questions to the AI provider about the conflicts of the review (`ai.resolve-conflicts`): one conflict per
//! question, two under way at a time, the others waiting their turn; the text of each answer as it comes, the answers
//! kept by repository, file and fingerprint until the window is closed, and the failures on their cards.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use engine::ai::ConflictRequest;
use model::ai::{AiSettings, QuestionInput, streamed_lines};
use model::review::{AiAnswer, ReviewFile};
use views::review::{CardAi, ConflictKey};

use super::super::feature::Said;
use super::super::message::{Effect, Message};
use super::super::state::State;
use super::{Fingerprinted, derive};
use crate::persisted::same_folder;
use crate::work::ai::{AiFailed, AiQuestion, AiReply, AiResult, AiWork, AskId};

/// How many questions about conflicts are under way at most.
pub const MOST_UNDER_WAY: usize = 2;

/// A question about a conflict, waiting or under way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// The folder of the repository of the conflict.
    pub folder: PathBuf,
    /// The conflict.
    pub conflict: Fingerprinted,
    /// The text of the answer so far, once some came.
    pub streamed: Option<String>,
    /// Whether it was sent; otherwise it waits its turn.
    pub is_sent: bool,
}

/// The questions about conflicts, and what came of them.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Questions {
    /// The number of the next question of the window.
    pub next: AskId,
    /// The questions waiting or under way.
    pub asked: BTreeMap<AskId, Asked>,
    /// The questions waiting their turn, in order.
    pub waiting: VecDeque<AiQuestion>,
    /// How many were asked since none was under way.
    pub total: usize,
    /// How many of them came to an end.
    pub answered: usize,
    /// The answers, by repository and file.
    pub answers: Vec<(PathBuf, String, AiAnswer)>,
    /// The conflicts a question failed on, by repository, with why.
    pub failed: Vec<(PathBuf, Fingerprinted, String)>,
}

/// The number of a new question of the window.
pub fn next_ask(state: &mut State) -> AskId {
    let questions = &mut state.reviews.questions;
    questions.next += 1;
    questions.next
}

/// The answers kept for the conflicts of a file of a repository.
#[must_use]
pub fn answers_of(questions: &Questions, folder: &Path, path: &str) -> Vec<AiAnswer> {
    questions
        .answers
        .iter()
        .filter(|(known, file, _)| same_folder(known, folder) && file == path)
        .map(|(_, _, answer)| answer.clone())
        .collect()
}

/// Where the AI stands on each conflict of a repository it was asked about.
#[must_use]
pub fn card_states(questions: &Questions, folder: &Path) -> BTreeMap<Fingerprinted, CardAi> {
    let mut states: BTreeMap<Fingerprinted, CardAi> = questions
        .failed
        .iter()
        .filter(|(known, _, _)| same_folder(known, folder))
        .map(|(_, conflict, why)| (conflict.clone(), CardAi::Failed(why.clone())))
        .collect();
    for asked in questions
        .asked
        .values()
        .filter(|asked| same_folder(&asked.folder, folder))
    {
        let state = match &asked.streamed {
            Some(text) => CardAi::Writing(streamed_lines(text)),
            None => CardAi::Reading,
        };
        let _before = states.insert(asked.conflict.clone(), state);
    }
    states
}

/// Asks the AI provider about one conflict of the review page, or about each conflict that has nothing proposed and
/// is not asked about already; with `conflicts.ai` off, refuses. Before the page is read, it asks once it is.
pub fn ask(state: &mut State, which: Option<ConflictKey>) -> Vec<Effect> {
    if !state.settings.features.conflicts_ai {
        refuse(state);
        return Vec::new();
    }
    let settings = (*state.settings.features.ai).clone();
    let reviews = &mut state.reviews;
    let Some(page) = reviews.page.as_mut() else {
        return Vec::new();
    };
    let Some(readout) = page.readout.as_deref() else {
        page.asks_once_read = true;
        return Vec::new();
    };
    let asked_already = card_states(&reviews.questions, &page.folder);
    let is_free = |key: &ConflictKey| {
        !matches!(
            asked_already.get(&(key.path.clone(), key.fingerprint.clone())),
            Some(CardAi::Reading | CardAi::Writing(_))
        )
    };
    let keys: Vec<ConflictKey> = match which {
        Some(key) => vec![key],
        None => page
            .cards
            .iter()
            .flat_map(|file| &file.conflicts)
            .filter(|card| card.proposal.is_none())
            .map(|card| card.key.clone())
            .collect(),
    };
    let built: Vec<(Fingerprinted, ConflictRequest)> = keys
        .into_iter()
        .filter(is_free)
        .filter_map(|key| {
            let file = readout.read.files.iter().find(|file| file.path == key.path)?;
            let request = request(file, &readout.read.sides, &key.fingerprint)?;
            Some(((key.path, key.fingerprint), request))
        })
        .collect();
    let folder = page.folder.clone();
    page.report = None;
    put_in_line(&mut reviews.questions, (&folder, &settings), built);
    derive(state);
    pump(state)
}

/// Puts questions in line after those waiting; with no AI provider set up, their cards say so at once.
fn put_in_line(
    questions: &mut Questions,
    (folder, settings): (&Path, &AiSettings),
    built: Vec<(Fingerprinted, ConflictRequest)>,
) {
    for (conflict, request) in built {
        questions
            .failed
            .retain(|(known, failed, _)| !(same_folder(known, folder) && *failed == conflict));
        if settings.provider.is_none() {
            let why = engine::ai::AiError::NotSetUp.to_string();
            questions.failed.push((folder.to_path_buf(), conflict, why));
            continue;
        }
        questions.next += 1;
        let id = questions.next;
        let _before = questions.asked.insert(
            id,
            Asked {
                folder: folder.to_path_buf(),
                conflict,
                streamed: None,
                is_sent: false,
            },
        );
        questions.waiting.push_back(AiQuestion {
            id,
            settings: settings.clone(),
            work: AiWork::Conflict(request),
        });
        questions.total += 1;
    }
}

/// **Resolve with AI**: opens the review page, of some files or of all, and asks about each conflict that has nothing
/// proposed once it is read; with `conflicts.ai` off, refuses.
pub fn resolve_with_ai(state: &mut State, only: Option<Vec<String>>) -> Vec<Effect> {
    if !state.settings.features.conflicts_ai {
        refuse(state);
        return Vec::new();
    }
    let mut effects = super::open(state, only);
    effects.extend(ask(state, None));
    effects
}

/// The question about a conflict of a file, with the lines around it its answer is checked against.
fn request(file: &ReviewFile, sides: &model::conflicts::SideNames, fingerprint: &str) -> Option<ConflictRequest> {
    let conflict = file
        .conflicts
        .iter()
        .find(|conflict| conflict.fingerprint == fingerprint)?;
    let lines: Vec<&str> = file.lines().into_iter().map(|(text, _)| text).collect();
    Some(ConflictRequest::of(&QuestionInput {
        path: &file.path,
        sides,
        lines: &lines,
        conflict,
    }))
}

/// Sends the questions waiting, while fewer than [`MOST_UNDER_WAY`] are under way.
fn pump(state: &mut State) -> Vec<Effect> {
    let questions = &mut state.reviews.questions;
    let mut effects = Vec::new();
    while questions.asked.values().filter(|asked| asked.is_sent).count() < MOST_UNDER_WAY {
        let Some(question) = questions.waiting.pop_front() else {
            break;
        };
        if let Some(asked) = questions.asked.get_mut(&question.id) {
            asked.is_sent = true;
        }
        effects.push(Effect::Ask(Box::new(question)));
    }
    effects
}

/// What a question to the AI provider says: the text of its answer so far, or what it came to, for a conflict or for
/// the draft of a commit message.
pub fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::AiPiece { ask, text } => piece(state, ask, text),
        Message::AiAnswered { ask, result } if state.reviews.questions.asked.contains_key(&ask) => {
            answered(state, ask, result)
        }
        Message::AiAnswered { ask, result } => super::super::draft::drafted(state, ask, result),
        _ => Vec::new(),
    }
}

/// Takes the text of an answer so far.
pub fn piece(state: &mut State, ask: AskId, text: String) -> Vec<Effect> {
    if let Some(asked) = state.reviews.questions.asked.get_mut(&ask) {
        asked.streamed = Some(text);
        derive(state);
    }
    Vec::new()
}

/// Takes what a question came to: its answer is kept in place of the one before, or its card says why it failed; a
/// model that is not there stops the questions waiting. Then sends the next.
pub fn answered(state: &mut State, ask: AskId, result: AiResult) -> Vec<Effect> {
    let questions = &mut state.reviews.questions;
    let Some(asked) = questions.asked.remove(&ask) else {
        return Vec::new();
    };
    questions.answered += 1;
    let (folder, (path, fingerprint)) = (asked.folder, asked.conflict);
    match result {
        Ok(AiReply::Conflict(reply)) => {
            questions.answers.retain(|(known, file, answer)| {
                !(same_folder(known, &folder) && *file == path && answer.fingerprint == fingerprint)
            });
            let answer = AiAnswer {
                fingerprint,
                model: reply.model,
                answer: reply.answer,
            };
            questions.answers.push((folder, path, answer));
        }
        Ok(AiReply::Draft(_)) => {}
        Err(AiFailed { text, stops_the_rest }) => {
            questions.failed.push((folder, (path, fingerprint), text));
            if stops_the_rest {
                for waiting in questions.waiting.drain(..) {
                    let _dropped = questions.asked.remove(&waiting.id);
                }
            }
        }
    }
    if questions.asked.is_empty() {
        questions.total = 0;
        questions.answered = 0;
    }
    derive(state);
    pump(state)
}

/// **Stop**: cancels the questions under way and drops the ones waiting; the cards that failed keep their error.
pub fn stop(state: &mut State) -> Vec<Effect> {
    let questions = &mut state.reviews.questions;
    let sent: Vec<AskId> = questions
        .asked
        .iter()
        .filter(|(_, asked)| asked.is_sent)
        .map(|(id, _)| *id)
        .collect();
    questions.asked.clear();
    questions.waiting.clear();
    questions.total = 0;
    questions.answered = 0;
    derive(state);
    if sent.is_empty() {
        Vec::new()
    } else {
        vec![Effect::StopAsking(sent)]
    }
}

/// With `conflicts.ai` off: *Resolving conflicts with AI is experimental, and it is switched off in the settings.*
/// with **Open settings**.
fn refuse(state: &mut State) {
    state.operations.said = Some(Said {
        text: String::from(views::text::review::AI_SWITCHED_OFF),
        is_error: true,
        can_undo: false,
        offers_stashes: false,
        found: None,
        offer: Some((
            model::contract::FeatureId("settings.edit"),
            String::from(views::text::review::OPEN_SETTINGS),
        )),
        stopped: None,
        runs: Vec::new(),
        link: None,
        shown_at: None,
        log_at: None,
    });
}
