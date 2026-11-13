//! The review of the conflicts as the store keeps it (`review.page`): the page, its read on a worker, read again
//! whenever the repository is, what is accepted on it, and the choices, kept while the window is open by file and
//! fingerprint of the conflict (`review.choose`).

mod cards;
mod intents;

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

/// A conflict by its file and its fingerprint, which stays the same while the file is read again.
pub type Fingerprinted = (String, String);

/// The review page, and what the window keeps of the review while it is open.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Reviews {
    /// The page, while it is open.
    pub page: Option<ReviewPage>,
    /// The choices made, by repository, file and fingerprint of the conflict.
    pub choices: Vec<(PathBuf, Fingerprinted, ChoiceKind)>,
    /// Cards or Columns, kept when the page is opened again.
    pub layout: Layout,
    /// The widths of the current and incoming columns, in thousandths, once a band was dragged.
    pub widths: Option<(u32, u32)>,
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
    if !state.settings.conflicts.auto_resolve && !state.settings.conflicts.ai {
        state.operations.said = Some(Said {
            text: String::from(views::text::review::SWITCHED_OFF),
            is_error: true,
            can_undo: false,
            offers_stashes: false,
            found: None,
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
    let mut effects = Vec::new();
    if let Some(job) = running {
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    let job = super::workspace::start_job(
        state,
        JobKind::ReadReview {
            folder,
            request: ReviewRequest { only },
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
            page.readout = Some(readout);
            page.is_failed = false;
        }
        Err(error) => {
            page.is_failed = true;
            state.last_error = Some(error);
        }
    }
    derive(state);
    Vec::new()
}

/// Derives again what each card shows.
fn derive(state: &mut State) {
    let settings = (state.settings.conflicts.auto_resolve, state.settings.conflicts.ai);
    let choices = state.reviews.choices.clone();
    if let Some(page) = state.reviews.page.as_mut() {
        let folder = page.folder.clone();
        let choices: Vec<(Fingerprinted, ChoiceKind)> = choices
            .into_iter()
            .filter(|(known, _, _)| same_folder(known, &folder))
            .map(|(_, key, kind)| (key, kind))
            .collect();
        page.cards = cards::cards(page, &choices, settings);
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
            rules: state.settings.conflicts.auto_resolve,
            ai: state.settings.conflicts.ai,
        },
        is_running: state.is_operation_running,
        can_continue: features::operation(intents::CONTINUE).is_some(),
    })
}
