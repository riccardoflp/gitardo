//! The comparison pages as the store keeps them (`compare.page`): the two sides of each and how they are compared, the
//! read on a worker, read again when the repository is and a side follows its name or is the working tree, the
//! comparisons opened before in each repository (`compare.recent`), and the comparison in the inspector
//! (`compare.in-place`).

pub mod beside;
pub mod chooser;
pub mod in_place;
mod intents;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use model::compare::{
    CompareSide, CompareView, ComparisonPart, ComparisonTarget, RecentComparison, RecentInput, RecentSide, Side,
    is_same_page, recent,
};
use model::state::RepositoryState;
use views::compare::{ComparePageId, CompareRead, CompareState};
use views::page::Page;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::compare::ComparisonRead;

pub use intents::reduce;

/// The comparison pages, the comparisons opened before and the comparison in the inspector.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Compares {
    /// The pages, in the order they were opened.
    pub pages: Vec<ComparePage>,
    /// The number of the next page.
    pub next: u64,
    /// The page whose side is being changed by `compare.change-side`, which the comparison it opens replaces.
    pub changing: Option<ComparePageId>,
    /// The comparisons opened before in each repository, the latest first.
    pub recent: Vec<(PathBuf, Vec<RecentComparison>)>,
    /// The comparison in the inspector, while it is shown.
    pub in_place: Option<in_place::InPlace>,
    /// The comparison chooser, while it is open (`compare.chooser`).
    pub chooser: Option<chooser::Chooser>,
}

/// How far a comparison page is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageRead {
    /// Not read yet.
    Comparing,
    /// Read.
    Read(Box<CompareView>),
    /// Git could not compare the two.
    Failed,
}

/// A comparison page.
#[derive(Clone, Debug, PartialEq)]
pub struct ComparePage {
    /// Its number.
    pub id: ComparePageId,
    /// The folder of its repository, as found in the workspace: the page stays of it when another is shown.
    pub folder: PathBuf,
    /// Its repository as last read, which the operations of the page run on.
    pub repository: Arc<RepositoryState>,
    /// The read of the repository it was last read against.
    pub generation: u64,
    /// The two sides, how they are compared and the path it is narrowed to.
    pub target: ComparisonTarget,
    /// Its title in the trail.
    pub title: String,
    /// How far it is read.
    pub read: PageRead,
    /// The read running.
    pub job: Option<JobId>,
    /// What is typed in the box that narrows it.
    pub typed: String,
    /// Whether **Show all N commits…** was clicked.
    pub is_all_commits_shown: bool,
    /// The files shown beside it, while one is (`compare.beside-diff`).
    pub beside: Option<beside::Beside>,
}

impl ComparePage {
    /// The comparison as a feature is invoked on a part of the page.
    #[must_use]
    pub fn target_of(&self, part: ComparisonPart) -> ComparisonTarget {
        let merge_base = match &self.read {
            PageRead::Read(view) => view.merge_base.clone(),
            PageRead::Comparing | PageRead::Failed => None,
        };
        ComparisonTarget {
            merge_base,
            part,
            ..self.target.clone()
        }
    }
}

/// A page, to change.
fn page_mut(state: &mut State, id: ComparePageId) -> Option<&mut ComparePage> {
    state.compares.pages.iter_mut().find(|page| page.id == id)
}

/// The title of a comparison page.
fn title_of(target: &ComparisonTarget) -> String {
    views::text::compare::title(&target.first.name, &target.second.name, target.path.as_deref())
}

/// The comparisons opened before in a repository, the latest first.
#[must_use]
pub fn recent_of(state: &State, folder: &Path) -> Vec<RecentComparison> {
    state
        .compares
        .recent
        .iter()
        .find(|(known, _)| same_folder(known, folder))
        .map_or_default(|(_, recent)| recent.clone())
}

/// Remembers a comparison just opened, swapped or narrowed as the latest of its repository.
fn remember(state: &mut State, folder: &Path, target: &ComparisonTarget) {
    let side = |side: &CompareSide| RecentSide {
        name: side.name.clone(),
        commit: match &side.side {
            Side::Commit(commit) => Some(commit.clone()),
            Side::WorkingTree => None,
        },
    };
    let opened = RecentComparison {
        first: side(&target.first),
        second: side(&target.second),
        path: target.path.clone(),
    };
    let remembered = recent_of(state, folder);
    let now = recent(&RecentInput {
        remembered: &remembered,
        opened: &opened,
    });
    let recent = &mut state.compares.recent;
    match recent.iter_mut().find(|(known, _)| same_folder(known, folder)) {
        Some((_, known)) => *known = now,
        None => recent.push((folder.to_owned(), now)),
    }
}

/// What `FollowUp::OpenComparison` opens: the page whose side was being changed compares the new pair, with the path it
/// had when both sides were chosen again; otherwise the page of the pair in the repository shown, opened once.
pub fn opened(state: &mut State, target: ComparisonTarget) -> Vec<Effect> {
    let target = ComparisonTarget {
        part: ComparisonPart::Whole,
        ..target
    };
    if let Some(id) = state.compares.changing.take()
        && let Some(page) = state.compares.pages.iter_mut().find(|page| page.id == id)
    {
        let target = ComparisonTarget {
            path: target.path.or_else(|| page.target.path.clone()),
            ..target
        };
        page.title = title_of(&target);
        page.target = target.clone();
        let folder = page.folder.clone();
        remember(state, &folder, &target);
        return read_again(state, id).into_iter().collect();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, repository, generation) = (read.folder.clone(), Arc::clone(&read.state), read.generation);
    open_page(state, (folder, repository, generation), target)
}

/// Opens the page of a comparison in a repository, or shows the one it has.
pub fn open_page(
    state: &mut State,
    (folder, repository, generation): (PathBuf, Arc<RepositoryState>, u64),
    target: ComparisonTarget,
) -> Vec<Effect> {
    remember(state, &folder, &target);
    let key = (&target.first, &target.second, target.path.as_deref());
    let found = state.compares.pages.iter().find(|page| {
        same_folder(&page.folder, &folder)
            && is_same_page(
                key,
                (&page.target.first, &page.target.second, page.target.path.as_deref()),
            )
    });
    if let Some(page) = found {
        super::pages::show_page(state, Page::Compare(page.id));
        return Vec::new();
    }
    let id = ComparePageId(state.compares.next);
    state.compares.next += 1;
    state.compares.pages.push(ComparePage {
        id,
        folder,
        repository,
        generation,
        title: title_of(&target),
        typed: target.path.clone().unwrap_or_default(),
        target,
        read: PageRead::Comparing,
        job: None,
        is_all_commits_shown: false,
        beside: None,
    });
    super::pages::show_page(state, Page::Compare(id));
    let mut effects: Vec<Effect> = read_again(state, id).into_iter().collect();
    effects.extend(super::pages::tick_commit_page(state));
    effects
}

/// Reads a page again, cancelling the read it had running.
fn read_again(state: &mut State, id: ComparePageId) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let (folder, target, running) = (page.folder.clone(), page.target.clone(), page.job);
    let mut effects = Vec::new();
    if let Some(job) = running {
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    let job = super::workspace::start_job(
        state,
        JobKind::ReadComparison {
            folder,
            target: Box::new(target),
        },
    );
    if let Some(page) = state.compares.pages.iter_mut().find(|page| page.id == id) {
        page.job = Some(job.id);
    }
    effects.push(Effect::Start(job));
    effects
}

/// Takes the read of a page: its sides where they are now, and what differs. The read of a page closed meanwhile is
/// dropped.
pub fn comparison_read(state: &mut State, job: JobId, result: Result<Box<ComparisonRead>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    if state
        .compares
        .chooser
        .as_ref()
        .is_some_and(|chooser| chooser.summary_job == Some(job))
    {
        chooser::summary_read(state, result);
        return Vec::new();
    }
    let Some(page) = state.compares.pages.iter_mut().find(|page| page.job == Some(job)) else {
        return Vec::new();
    };
    let id = page.id;
    page.job = None;
    match result {
        Ok(read) => {
            let ComparisonRead { first, second, view } = *read;
            page.target.first = first;
            page.target.second = second;
            page.read = PageRead::Read(Box::new(view));
        }
        Err(error) => {
            page.read = PageRead::Failed;
            state.last_error = Some(error);
        }
    }
    beside::follow_read(state, id)
}

/// Forgets the pages closed, and cancels their reads.
pub fn prune(state: &mut State) -> Vec<Effect> {
    let open = &state.pages.open;
    let (kept, closed): (Vec<ComparePage>, Vec<ComparePage>) = std::mem::take(&mut state.compares.pages)
        .into_iter()
        .partition(|page| open.contains(&Page::Compare(page.id)));
    state.compares.pages = kept;
    closed
        .into_iter()
        .filter_map(|page| page.job)
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect()
}

/// Once the repository of a page is read again: the page keeps it, and is compared again when a side is the working
/// tree or follows its name. The comparison in the inspector goes when another repository is shown.
pub fn follow(state: &mut State) -> Vec<Effect> {
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, repository, generation) = (read.folder.clone(), Arc::clone(&read.state), read.generation);
    if state
        .compares
        .in_place
        .as_ref()
        .is_some_and(|in_place| !same_folder(&in_place.folder, &folder))
    {
        state.compares.in_place = None;
    }
    let to_read: Vec<ComparePageId> = state
        .compares
        .pages
        .iter_mut()
        .filter(|page| page.generation != generation && same_folder(&page.folder, &folder))
        .filter_map(|page| {
            page.generation = generation;
            page.repository = Arc::clone(&repository);
            let follows = |side: &CompareSide| side.is_followed || side.is_working_tree();
            (follows(&page.target.first) || follows(&page.target.second)).then_some(page.id)
        })
        .collect();
    to_read.into_iter().flat_map(|id| read_again(state, id)).collect()
}

/// What a comparison page shows.
#[must_use]
pub fn page_state(state: &State, id: ComparePageId) -> Option<CompareState<'_>> {
    let page = state.compares.pages.iter().find(|page| page.id == id)?;
    Some(CompareState {
        id,
        title: &page.title,
        target: &page.target,
        read: match &page.read {
            PageRead::Comparing => CompareRead::Comparing,
            PageRead::Read(view) => CompareRead::Read(view),
            PageRead::Failed => CompareRead::Failed,
        },
        is_reading: page.job.is_some(),
        typed_path: &page.typed,
        is_all_commits_shown: page.is_all_commits_shown,
        now: state.timeline.read_at,
        is_on_github: state.github.repository.is_some()
            && state
                .github
                .folder
                .as_ref()
                .is_some_and(|folder| same_folder(folder, &page.folder)),
    })
}
