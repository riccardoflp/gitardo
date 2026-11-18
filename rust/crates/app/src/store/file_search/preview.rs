//! The preview of the Search page as the store keeps it (`file-search.preview`): the file of the result under the
//! keyboard, read on a worker when that result comes under it, the preview before staying until it is read; or the
//! folder, from the files listed.

use std::sync::Arc;

use model::file_search::{ResultRow, Search};
use views::file_search::{PreviewShown, PreviewState};
use views::page::Page;

use super::FileSearchPage;
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;
use crate::work::file_search::{PreviewJob, PreviewRead, SearchJob};

/// The preview of the Search page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Preview {
    /// The file shown, what it was read for and what was read.
    pub shown: Option<(PreviewKey, Arc<PreviewRead>)>,
    /// The read running: its job and what it reads.
    pub reading: Option<(JobId, PreviewKey)>,
    /// The folder shown: its path, then the folders and the files it holds, by their names.
    pub folder: Option<(String, Vec<String>, Vec<String>)>,
    /// The file and the line last brought to the middle.
    pub revealed: Option<(String, Option<usize>)>,
    /// A number that changes each time the line lit is to be brought to the middle.
    pub reveal: u64,
}

/// What a file is read for: its path, what is looked for in it when its matches are marked, and the read of the
/// repository it was read after.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewKey {
    /// Its path from the top of the worktree.
    pub path: String,
    /// What is looked for, with only what changes the matches.
    pub search: Option<Search>,
    /// The read of the repository.
    pub generation: u64,
}

/// What the result under the keyboard asks the preview to show.
enum Wanted {
    /// A file, at a line found in it, from one.
    File(String, Option<usize>),
    /// A folder.
    Folder(String),
}

/// What the result under the keyboard asks the preview to show; nothing on a heading, or with no result.
fn wanted(page: &FileSearchPage) -> Option<Wanted> {
    let found = &page.found;
    let line_of = |index: usize| {
        found
            .text
            .get(index)
            .map(|line| Wanted::File(line.path.clone(), Some(line.line)))
    };
    match page.rows.get(page.lit?)? {
        ResultRow::File(index) => Some(Wanted::File(found.files.get(*index)?.path.clone(), None)),
        ResultRow::Folder(index) => Some(Wanted::Folder(found.folders.get(*index)?.path.clone())),
        ResultRow::TextFile { first, .. } => line_of(*first),
        ResultRow::Line(index) => line_of(*index),
        ResultRow::Heading(_) => None,
    }
}

/// Reads the file of the result under the keyboard while the page is shown, unless it is shown or being read already,
/// the read before stopped; shows a folder from the files listed; and brings the line found to the middle once its
/// file is shown.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if state.page_shown() != Some(Page::Search) {
        return Vec::new();
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.state.folder.clone(), read.generation);
    let page = &mut state.file_search;
    let (path, line) = match wanted(page) {
        None => return Vec::new(),
        Some(Wanted::Folder(path)) => {
            show_folder(page, path);
            return Vec::new();
        }
        Some(Wanted::File(path, line)) => (path, line),
    };
    page.preview.folder = None;
    reveal(&mut page.preview, &path, line);
    let key = PreviewKey {
        path,
        search: marked_for(&page.search),
        generation,
    };
    let preview = &page.preview;
    let is_shown = preview.shown.as_ref().is_some_and(|(shown, _)| *shown == key);
    if is_shown || preview.reading.as_ref().is_some_and(|(_, reading)| *reading == key) {
        return Vec::new();
    }
    let mut effects = stop(state);
    let job = SearchJob::Preview(Box::new(PreviewJob {
        path: key.path.clone(),
        search: key.search.clone(),
    }));
    let job = crate::store::workspace::start_job(state, JobKind::FileSearch { folder, job });
    state.file_search.preview.reading = Some((job.id, key));
    effects.push(Effect::Start(job));
    effects
}

/// What the matches of a file are marked for: what is looked for, with only what changes them, when the text is
/// searched; nothing otherwise.
fn marked_for(search: &Search) -> Option<Search> {
    let is_searched = search.mode.searches_text() && !search.query.trim().is_empty();
    is_searched.then(|| Search {
        query: search.query.clone(),
        match_case: search.match_case,
        whole_word: search.whole_word,
        regex: search.regex,
        ..Search::default()
    })
}

/// Brings the line found to the middle, or the first line of a file found by its name, once the file is shown and
/// each time another result of it comes under the keyboard.
fn reveal(preview: &mut Preview, path: &str, line: Option<usize>) {
    let is_shown = preview.shown.as_ref().is_some_and(|(shown, _)| shown.path == path);
    let target = (path.to_owned(), line);
    if is_shown && preview.revealed.as_ref() != Some(&target) {
        preview.revealed = Some(target);
        preview.reveal += 1;
    }
}

/// Shows a folder: the folders, then the files right under it, from those listed.
fn show_folder(page: &mut FileSearchPage, path: String) {
    if page.preview.folder.as_ref().is_some_and(|(shown, ..)| *shown == path) {
        return;
    }
    let Some((names, _)) = &page.names else {
        return;
    };
    let prefix = format!("{path}/");
    let under = |names: &[String]| -> Vec<String> {
        let names = names.iter().filter_map(|name| name.strip_prefix(&prefix));
        names.filter(|name| !name.contains('/')).map(str::to_owned).collect()
    };
    let (folders, files) = (under(&names.folders), under(&names.files));
    page.preview.folder = Some((path, folders, files));
}

/// Stops the read of the preview running.
pub fn stop(state: &mut State) -> Vec<Effect> {
    let Some((job, _)) = state.file_search.preview.reading.take() else {
        return Vec::new();
    };
    let _cancelled = state.jobs.remove(&job);
    vec![Effect::Cancel(job)]
}

/// Takes a file read for the preview, unless another was asked for since.
pub fn previewed(page: &mut FileSearchPage, job: JobId, read: Arc<PreviewRead>) {
    let preview = &mut page.preview;
    let Some((_, key)) = preview.reading.take_if(|(reading, _)| *reading == job) else {
        return;
    };
    preview.shown = Some((key, read));
    preview.revealed = None;
}

/// What the preview is drawn from: the folder or the file of the result under the keyboard, or the one before it while
/// it is read; nothing before the first.
#[must_use]
pub fn preview_state(state: &State) -> Option<PreviewState<'_>> {
    let page = &state.file_search;
    let preview = &page.preview;
    let lit_line = match wanted(page) {
        Some(Wanted::Folder(_)) => {
            let (path, folders, files) = preview.folder.as_ref()?;
            return Some(PreviewState {
                path,
                shown: PreviewShown::Folder { folders, files },
                lit_line: None,
                reveal: (preview.reveal, 0),
                tab_width: crate::store::file_page::TAB_WIDTH,
                wrap: state.settings.word_wrap,
            });
        }
        Some(Wanted::File(path, line)) => {
            line.filter(|_| preview.shown.as_ref().is_some_and(|(key, _)| key.path == path))
        }
        None => None,
    };
    let (key, read) = preview.shown.as_ref()?;
    let shown = match read.as_ref() {
        PreviewRead::Text { buffer, colors, marks } => PreviewShown::Text { buffer, colors, marks },
        PreviewRead::NotText => PreviewShown::NotText,
        PreviewRead::TooLarge => PreviewShown::TooLarge,
        PreviewRead::Failed(said) => PreviewShown::Failed(said),
    };
    let lit_line = lit_line.map(|line| line.saturating_sub(1));
    Some(PreviewState {
        path: &key.path,
        shown,
        lit_line,
        reveal: (preview.reveal, lit_line.unwrap_or(0)),
        tab_width: crate::store::file_page::TAB_WIDTH,
        wrap: state.settings.word_wrap,
    })
}
