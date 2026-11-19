//! The review of the conflicts as the store keeps it (`review.page`): the page, its read on a worker, read again
//! whenever the repository is, what is accepted on it, and the choices, kept while the window is open by file and
//! fingerprint of the conflict (`review.choose`).

pub mod ai;
mod cards;
mod intents;
mod lines;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use model::review::ChoiceKind;
use model::state::RepositoryState;
use views::page::Page;
use views::review::{ConflictKey, FileCards, Filter, Layout, Report, ReviewRead, ReviewState};

use super::feature::Said;
use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::review::{ReviewReadout, ReviewRequest};

pub use intents::{ended, reduce};
pub use lines::{LinesBox, box_state, written_of};

/// A conflict by its file and its fingerprint, which stays the same while the file is read again.
pub type Fingerprinted = (String, String);

/// The review page, and what the window keeps of the review while it is open.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Reviews {
    /// The page, while it is open.
    pub page: Option<ReviewPage>,
    /// The choices made, by repository, file and fingerprint of the conflict.
    pub choices: Vec<(PathBuf, Fingerprinted, ChoiceKind)>,
    /// The lines written for the conflicts whose choice is **Edit**, the same way.
    pub written: Vec<(PathBuf, Fingerprinted, Vec<String>)>,
    /// The box **Edit** opened on a card, while it is open.
    pub lines: Option<LinesBox>,
    /// How many times a box was opened, which gives each the keyboard once.
    pub opened: u64,
    /// Cards or Columns, kept when the page is opened again.
    pub layout: Layout,
    /// The widths of the current and incoming columns, in thousandths, once a band was dragged.
    pub widths: Option<(u32, u32)>,
    /// The questions to the AI provider about conflicts, and their answers (`ai.resolve-conflicts`).
    pub questions: ai::Questions,
}

/// The review page.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewPage {
    /// The folder of its repository.
    pub folder: PathBuf,
    /// Its repository as last read.
    pub repository: Arc<RepositoryState>,
    /// The read of the repository it was last read against.
    pub generation: u64,
    /// Its title in the trail.
    pub title: String,
    /// The files it is about, when not every file in conflict.
    pub only: Option<Vec<String>>,
    /// What was read, once it was.
    pub readout: Option<Box<ReviewReadout>>,
    /// Whether the last read failed.
    pub is_failed: bool,
    /// The read running.
    pub job: Option<JobId>,
    /// The conflicts accepted.
    pub accepted: BTreeSet<Fingerprinted>,
    /// The files git resolved as before that are accepted (`review.recorded`).
    pub recorded_accepted: Vec<String>,
    /// The conflicts folded or unfolded by hand.
    pub folds: BTreeMap<Fingerprinted, bool>,
    /// The files folded or unfolded by hand.
    pub file_folds: BTreeMap<String, bool>,
    /// The cards whose sides are shown.
    pub sides_shown: BTreeSet<Fingerprinted>,
    /// The cards whose base is shown.
    pub base_shown: BTreeSet<Fingerprinted>,
    /// The filter chosen.
    pub filter: Filter,
    /// The conflicts accepted while **Not accepted** was on.
    pub kept_in_view: Vec<ConflictKey>,
    /// The file shown alone.
    pub file_shown: Option<String>,
    /// What the last write did.
    pub report: Option<Report>,
    /// What the write running will have done, once it ran.
    pub writing: Option<Report>,
    /// What each card shows, derived from the rest.
    pub cards: Vec<FileCards>,
    /// Whether the AI provider is asked about the conflicts with nothing proposed once the page is read.
    pub asks_once_read: bool,
}

/// The title of the page: **Conflicts · \<folder name\>**.
fn title_of(folder: &Path) -> String {
    let name = folder.file_name().map_or_else(
        || folder.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    views::text::review::title(&name)
}

/// Opens the review page of the repository shown, of some files or of all, or shows it with those files; refuses
/// while neither `conflicts.autoResolve` nor `conflicts.ai` is on.
pub fn open(state: &mut State, only: Option<Vec<String>>) -> Vec<Effect> {
    if !state.settings.features.conflicts_auto_resolve && !state.settings.features.conflicts_ai {
        state.operations.said = Some(Said {
            text: String::from(views::text::review::SWITCHED_OFF),
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
        return Vec::new();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, repository, generation) = (read.folder.clone(), Arc::clone(&read.state), read.generation);
    let is_same = state
        .reviews
        .page
        .as_ref()
        .is_some_and(|page| same_folder(&page.folder, &folder) && page.only == only);
    if !is_same {
        state.reviews.page = Some(ReviewPage {
            title: title_of(&folder),
            folder,
            repository,
            generation,
            only,
            readout: None,
            is_failed: false,
            job: None,
            accepted: BTreeSet::new(),
            recorded_accepted: Vec::new(),
            folds: BTreeMap::new(),
            file_folds: BTreeMap::new(),
            sides_shown: BTreeSet::new(),
            base_shown: BTreeSet::new(),
            filter: Filter::All,
            kept_in_view: Vec::new(),
            file_shown: None,
            report: None,
            writing: None,
            cards: Vec::new(),
            asks_once_read: false,
        });
    }
    super::pages::show_page(state, Page::Review);
    if is_same { Vec::new() } else { read_again(state) }
}

/// Reads the page again, cancelling the read it had running.
fn read_again(state: &mut State) -> Vec<Effect> {
    let Some(page) = state.reviews.page.as_ref() else {
        return Vec::new();
    };
    let (folder, only, running) = (page.folder.clone(), page.only.clone(), page.job);
    let recorded = page
        .repository
        .resolved_as_before
        .iter()
        .filter(|file| !file.is_staged)
        .map(|file| file.path.clone())
        .collect();
    let mut effects = Vec::new();
    if let Some(job) = running {
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    let job = super::workspace::start_job(
        state,
        JobKind::ReadReview {
            folder,
            request: ReviewRequest { only, recorded },
        },
    );
    if let Some(page) = state.reviews.page.as_mut() {
        page.job = Some(job.id);
    }
    effects.push(Effect::Start(job));
    effects
}

/// Takes the read of the page; the read of a page closed meanwhile is dropped.
pub fn review_read(state: &mut State, job: JobId, result: Result<Box<ReviewReadout>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(page) = state.reviews.page.as_mut().filter(|page| page.job == Some(job)) else {
        return Vec::new();
    };
    page.job = None;
    match result {
        Ok(readout) => {
            let recorded = &readout.read.recorded;
            page.recorded_accepted
                .retain(|path| recorded.iter().any(|file| file.path == *path));
            page.readout = Some(readout);
            page.is_failed = false;
        }
        Err(error) => {
            page.is_failed = true;
            state.last_error = Some(error);
        }
    }
    let asks = page.readout.is_some() && std::mem::take(&mut page.asks_once_read);
    derive(state);
    if asks { ai::ask(state, None) } else { Vec::new() }
}

/// The choices made in the repository of the page, each with the lines written for it.
fn chosen(state: &State) -> Vec<cards::Chosen> {
    let Some(folder) = state.reviews.page.as_ref().map(|page| &page.folder) else {
        return Vec::new();
    };
    state
        .reviews
        .choices
        .iter()
        .filter(|(known, _, _)| same_folder(known, folder))
        .map(|(_, key, kind)| (key.clone(), *kind, written_of(state, key).to_vec()))
        .collect()
}

/// Derives again what each card shows.
fn derive(state: &mut State) {
    let settings = (
        state.settings.features.conflicts_auto_resolve,
        state.settings.features.conflicts_ai,
    );
    let choices = chosen(state);
    let questions = &state.reviews.questions;
    if let Some(page) = state.reviews.page.as_mut() {
        let folder = page.folder.clone();
        let known = cards::Known {
            choices: &choices,
            questions,
            ai_states: &ai::card_states(questions, &folder),
        };
        page.cards = cards::cards(page, &known, settings);
    }
}

/// Forgets the page once it is closed, and cancels its read.
pub fn prune(state: &mut State) -> Vec<Effect> {
    if state.pages.open.contains(&Page::Review) {
        return Vec::new();
    }
    let Some(page) = state.reviews.page.take() else {
        return Vec::new();
    };
    page.job
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .into_iter()
        .collect()
}

/// Once the repository of the page is read again, the page reads its files again.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, repository, generation) = (read.folder.clone(), Arc::clone(&read.state), read.generation);
    let Some(page) = state.reviews.page.as_mut() else {
        return Vec::new();
    };
    if page.generation == generation || !same_folder(&page.folder, &folder) {
        return Vec::new();
    }
    page.generation = generation;
    page.repository = repository;
    read_again(state)
}

/// What the review page shows.
#[must_use]
pub fn page_state(state: &State) -> Option<ReviewState<'_>> {
    let page = state.reviews.page.as_ref()?;
    let readout = page.readout.as_deref();
    Some(ReviewState {
        title: &page.title,
        read: match (readout, page.is_failed) {
            (_, true) => ReviewRead::Failed,
            (Some(_), false) => ReviewRead::Read,
            (None, false) => ReviewRead::Reading,
        },
        operation: readout.and_then(|readout| readout.read.operation),
        sides: readout.map_or(&cards::NO_SIDES, |readout| &readout.read.sides),
        files: &page.cards,
        deleted: readout.map_or(&[], |readout| &readout.read.deleted),
        recorded: readout.map_or(&[], |readout| &readout.read.recorded),
        recorded_accepted: &page.recorded_accepted,
        others: readout.map_or(&[], |readout| &readout.read.others),
        filter: page.filter,
        kept_in_view: &page.kept_in_view,
        file_shown: page.file_shown.as_deref(),
        layout: state.reviews.layout,
        widths: state
            .reviews
            .widths
            .unwrap_or((views::review::EVEN, views::review::EVEN)),
        report: page.report.as_ref(),
        proposers: model::review::Proposers {
            rules: state.settings.features.conflicts_auto_resolve,
            ai: state.settings.features.conflicts_ai,
        },
        is_running: state.is_operation_running,
        writing: box_state(state),
        can_continue: features::operation(intents::CONTINUE).is_some(),
        asking: (state.reviews.questions.total > 0)
            .then_some((state.reviews.questions.answered, state.reviews.questions.total)),
        model: state
            .settings
            .features
            .conflicts_ai
            .then(|| views::text::review::model_button(&state.settings.features.ai)),
    })
}
