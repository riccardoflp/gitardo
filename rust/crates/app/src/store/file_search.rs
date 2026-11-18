//! The Search page as the store keeps it (`file-search.page`): what is looked for and where, the files of the worktree
//! shown, listed on a worker when the page is shown and again each time the repository is read or a filter changes,
//! and what a search of them found, started again each time what is looked for changes, the search before stopped.

mod answers;

use std::path::PathBuf;
use std::sync::Arc;

use git::read::file_search::NamesRequest;
use model::file_search::{Asked, Found, Mode, ResultRow, Scope, Search, list_results, open_with, pathspecs};
use views::file_search::{FileSearchIntent, SearchPageState, Status, Switch};
use views::file_view::{FileTarget, FileViewIntent};
use views::page::Page;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::file_search::{FindJob, Names, SearchJob};

pub(super) use answers::answered;

/// The Search page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileSearchPage {
    /// What is looked for, and where.
    pub search: Search,
    /// The top folder of the worktree searched.
    pub folder: Option<PathBuf>,
    /// The files listed, and the read of the repository they were listed after.
    pub names: Option<(Arc<Names>, u64)>,
    /// The list of the files running: its job, what it lists and the read of the repository it lists after.
    pub listing: Option<(JobId, NamesRequest, u64)>,
    /// What git said when it could not list the files: nothing is searched until they are listed again.
    pub failed: Option<String>,
    /// The search running.
    pub finding: Option<Finding>,
    /// What the results shown were searched for; none before the first search.
    pub found_for: Option<Wanted>,
    /// What was found.
    pub found: Found,
    /// The rows of the results.
    pub rows: Vec<ResultRow>,
    /// The row under the keyboard.
    pub lit: Option<usize>,
    /// What git said of a regular expression it could not read.
    pub refused: Option<String>,
    /// A number that changes each time the box takes the keyboard.
    pub focus: u64,
    /// A number that changes each time the row under the keyboard is to be brought into sight.
    pub reveal: u64,
}

/// What a search is for: what is looked for, and the read of the repository whose files are searched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// What is looked for, and where.
    pub search: Search,
    /// The read of the repository.
    pub generation: u64,
}

/// A search running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Its job.
    pub job: JobId,
    /// What it is for.
    pub wanted: Wanted,
    /// What it found so far, kept apart until it ends when the results shown are for the same search: the repository
    /// was read again, and the results stay as they are until the new ones are whole. None when the results are
    /// shown as they come.
    pub kept_apart: Option<Found>,
}

/// Something asked of the Search page.
pub fn reduce(state: &mut State, intent: FileSearchIntent) -> Vec<Effect> {
    let page = &mut state.file_search;
    match intent {
        FileSearchIntent::Show => return show(state, None),
        FileSearchIntent::Here(place) => return show(state, Some(Asked::Here(place))),
        FileSearchIntent::Type(text) => page.search.query = text,
        FileSearchIntent::Turn(switch) => turn(&mut page.search, switch),
        FileSearchIntent::Mode(mode) => page.search.mode = mode,
        FileSearchIntent::Scope(scope) => page.search.scope = scope,
        FileSearchIntent::DropScope => {
            page.search.named = None;
            if page.search.scope == Scope::Named {
                page.search.scope = Scope::Repository;
            }
        }
        FileSearchIntent::Include(text) => page.search.include = text,
        FileSearchIntent::Exclude(text) => page.search.exclude = text,
        FileSearchIntent::Light(row) => {
            page.lit = Some(row);
            page.reveal += 1;
        }
        FileSearchIntent::Open(row) => return open(state, row),
    }
    Vec::new()
}

/// Shows the page as it was left, with the keyboard in its box; what it looks for and where follow how it was asked
/// for (`file-search.open`). The way to it starts again from the timeline.
fn show(state: &mut State, asked: Option<Asked>) -> Vec<Effect> {
    let asked = asked.unwrap_or(Asked::Page { selected: None });
    state.file_search.search = open_with(&state.file_search.search, &asked);
    state.file_search.focus += 1;
    super::pages::starting_again(state, |state| super::pages::show(state, Page::Search))
}

/// Turns a switch of the page.
fn turn(search: &mut Search, switch: Switch) {
    let turned = match switch {
        Switch::MatchCase => &mut search.match_case,
        Switch::WholeWord => &mut search.whole_word,
        Switch::Regex => &mut search.regex,
        Switch::Gitignore => &mut search.gitignore,
        Switch::Hidden => &mut search.hidden,
    };
    *turned = !*turned;
}

/// Opens a result: the File page of a file, at the line of a line of text, the Search page before it on the way to
/// it; a folder is shown in the tree of the Files page, opened.
fn open(state: &mut State, row: usize) -> Vec<Effect> {
    let page = &mut state.file_search;
    page.lit = Some(row);
    let found = &page.found;
    let (path, line) = match page.rows.get(row) {
        Some(ResultRow::File(index)) => (found.files.get(*index).map(|file| file.path.clone()), None),
        Some(ResultRow::TextFile { first, .. }) => (found.text.get(*first).map(|line| line.path.clone()), None),
        Some(ResultRow::Line(index)) => {
            let line = found.text.get(*index);
            (line.map(|line| line.path.clone()), line.map(|line| line.line))
        }
        Some(ResultRow::Folder(index)) => {
            let folder = found.folders.get(*index).map(|folder| folder.path.clone());
            return folder.map_or_default(|folder| show_folder(state, &folder));
        }
        Some(ResultRow::Heading(_)) | None => (None, None),
    };
    let Some(path) = path else {
        return Vec::new();
    };
    let target = FileTarget {
        path,
        revision: None,
        line,
    };
    super::file_page::reduce(state, FileViewIntent::Open(target))
}

/// Shows a folder in the tree of the Files page: the folders that hold it and the folder itself opened.
fn show_folder(state: &mut State, folder: &str) -> Vec<Effect> {
    let mut path = String::new();
    for name in folder.split('/') {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(name);
        let _new = state.files_page.opened.insert(path.clone());
    }
    super::files_page::reduce(state, views::files::FilesPageIntent::Open)
}

/// Lists the files again while the page is shown and the repository was read since they were listed, or what narrows
/// them changed; then searches again when what is looked for is not what the results are for. A page that is open
/// and not shown waits until it is shown; one closed stops what it runs.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if !state.pages.open.contains(&Page::Search) {
        return cancel(state);
    }
    if state.page_shown() != Some(Page::Search) {
        return Vec::new();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.state.folder.clone(), read.generation);
    let page = &mut state.file_search;
    if !page.folder.as_deref().is_some_and(|known| same_folder(known, &folder)) {
        let search = std::mem::take(&mut page.search);
        let (focus, reveal) = (page.focus, page.reveal);
        *page = FileSearchPage {
            search,
            folder: Some(folder.clone()),
            focus,
            reveal,
            ..FileSearchPage::default()
        };
    }
    let request = names_request(&page.search);
    let listed = page
        .names
        .as_ref()
        .filter(|(names, listed_after)| names.request == request && *listed_after == generation)
        .map(|(names, _)| Arc::clone(names));
    match listed {
        Some(names) => find(state, (folder, generation), names),
        None => list(state, (folder, generation), request),
    }
}

/// What lists the files a search looks in: the folder **Search here** named, or the file it named alone; otherwise
/// the worktree, narrowed by **Include**, **Exclude** and **Hidden**.
fn names_request(search: &Search) -> NamesRequest {
    let ignored = !search.gitignore;
    match search.place() {
        Some(place) if place.is_file => NamesRequest {
            folder: None,
            pathspecs: vec![format!(":(top,literal){}", place.path)],
            ignored,
        },
        place => NamesRequest {
            folder: place.map(|place| place.path.clone()),
            pathspecs: pathspecs(&search.filters()),
            ignored,
        },
    }
}

/// Lists the files, unless the list running is the one wanted; the list and the search running before are stopped.
fn list(state: &mut State, (folder, generation): (PathBuf, u64), request: NamesRequest) -> Vec<Effect> {
    let page = &mut state.file_search;
    let is_running = page
        .listing
        .as_ref()
        .is_some_and(|(_, running, listed_after)| *running == request && *listed_after == generation);
    if is_running {
        return Vec::new();
    }
    let mut effects = stop(state);
    state.file_search.failed = None;
    let job = SearchJob::Names(request.clone());
    let job = super::workspace::start_job(state, JobKind::FileSearch { folder, job });
    state.file_search.listing = Some((job.id, request, generation));
    effects.push(Effect::Start(job));
    effects
}

/// Searches the files listed, unless the results shown or the search running are for what is looked for now; the
/// search running before is stopped. The results stay until the new ones are whole when only the repository was read
/// again, and come as they are found when what is looked for changed.
fn find(state: &mut State, (folder, generation): (PathBuf, u64), names: Arc<Names>) -> Vec<Effect> {
    let page = &state.file_search;
    if page.failed.is_some() {
        return Vec::new();
    }
    let is_wanted = |wanted: &Wanted| wanted.generation == generation && wanted.search == page.search;
    if page.finding.as_ref().is_some_and(|finding| is_wanted(&finding.wanted)) {
        return Vec::new();
    }
    if page.finding.is_none() && page.found_for.as_ref().is_some_and(is_wanted) {
        return Vec::new();
    }
    let is_read_again = page.found_for.as_ref().is_some_and(|shown| shown.search == page.search);
    let wanted = Wanted {
        search: page.search.clone(),
        generation,
    };
    let changed = (wanted.search.scope == Scope::Changes).then(|| changed_files(state));
    let mut effects = stop(state);
    let job = SearchJob::Find(Box::new(FindJob {
        search: wanted.search.clone(),
        names,
        changed,
    }));
    let job = super::workspace::start_job(state, JobKind::FileSearch { folder, job });
    state.file_search.finding = Some(Finding {
        job: job.id,
        wanted,
        kept_apart: is_read_again.then(Found::default),
    });
    effects.push(Effect::Start(job));
    effects
}

/// The files that differ from the last commit, the untracked ones and those in conflict too.
fn changed_files(state: &State) -> Vec<String> {
    let Some(list) = super::changes::listed(state) else {
        return Vec::new();
    };
    let rows = list.rows.into_iter().map(|row| row.path);
    rows.chain(list.conflicts.into_iter().map(|file| file.path)).collect()
}

/// Stops the list and the search running.
fn stop(state: &mut State) -> Vec<Effect> {
    let page = &mut state.file_search;
    let listing = page.listing.take().map(|(job, ..)| job);
    let finding = page.finding.take().map(|finding| finding.job);
    let mut effects = Vec::new();
    for job in listing.into_iter().chain(finding) {
        let _cancelled = state.jobs.remove(&job);
        effects.push(Effect::Cancel(job));
    }
    effects
}

/// Stops what the page runs once it is closed, and forgets what it listed and found: it lists and searches again when
/// it is opened again. What is looked for stays.
fn cancel(state: &mut State) -> Vec<Effect> {
    let effects = stop(state);
    let page = &mut state.file_search;
    if page.names.is_some() || page.found_for.is_some() {
        let search = std::mem::take(&mut page.search);
        *page = FileSearchPage {
            search,
            focus: page.focus,
            reveal: page.reveal,
            ..FileSearchPage::default()
        };
    }
    effects
}

/// Puts the rows of the results in order again after what was found changed, and keeps the keyboard on a result: the
/// row it was on while it is still one, else the first result.
fn list_rows(page: &mut FileSearchPage, mode: Mode, keeps_lit: bool) {
    page.rows = list_results(&page.found, mode);
    let is_result = |row: usize| page.rows.get(row).is_some_and(|row| row.is_result());
    if !keeps_lit || !page.lit.is_some_and(is_result) {
        page.lit = page.rows.iter().position(|row| row.is_result());
        page.reveal += 1;
    }
}

/// What the page is drawn from.
#[must_use]
pub fn page_state(state: &State) -> SearchPageState<'_> {
    let page = &state.file_search;
    let status = match (&page.failed, &page.names) {
        (Some(error), _) => Status::Failed(error),
        (None, None) => Status::Reading,
        (None, Some(_)) if page.finding.is_some() || page.listing.is_some() => Status::Searching,
        (None, Some(_)) => Status::Done,
    };
    SearchPageState {
        search: &page.search,
        found: &page.found,
        rows: &page.rows,
        lit: page.lit,
        status,
        refused: page.refused.as_deref(),
        focus: page.focus,
        reveal: page.reveal,
    }
}
