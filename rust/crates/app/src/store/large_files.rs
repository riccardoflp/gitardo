//! The Large files page as the store keeps it (`lfs.page`): what was read of Git LFS and the files it stores for the
//! repository shown, read again once the repository was read since, on **Refresh**, and for another repository; the
//! locks of the server only when the page opens, on **Refresh** and once **Lock** or **Unlock** ended
//! (`lfs.read-locks`, Q1195).

use std::path::PathBuf;

use views::lfs::{LargeFilesIntent, LfsPageState};
use views::page::Page;

use super::message::{Effect, JobId, JobKind};
use super::state::State;
use crate::persisted::same_folder;
use crate::work::large_files::LargeFilesRead;

/// The Large files page.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct LargeFilesPage {
    /// The top folder of the working tree it shows.
    pub folder: Option<PathBuf>,
    /// The read of the repository the files were read after.
    pub generation: Option<u64>,
    /// What was read, or what git said; none before the first read.
    pub read: Option<Result<LargeFilesRead, String>>,
    /// The read running.
    pub job: Option<JobId>,
    /// Whether the files are read again once the read running ends.
    pub is_read_again: bool,
    /// When the server is asked for the locks.
    pub locks: LockReads,
}

/// When the page asks the server for the locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LockReads {
    /// Whether the next read asks for them.
    pub is_wanted: bool,
    /// Whether the read running asks for them.
    pub is_reading: bool,
    /// Whether they are read again once the operation running ends: **Lock** or **Unlock** was chosen.
    pub after_operation: bool,
}

/// Something asked of the page.
pub fn reduce(state: &mut State, intent: LargeFilesIntent) -> Vec<Effect> {
    match intent {
        LargeFilesIntent::Open => open(state),
        LargeFilesIntent::OpenForTab(index) => {
            let mut effects = super::tabs::switch_tab(state, index);
            effects.extend(open(state));
            effects
        }
        LargeFilesIntent::Refresh => {
            state.large_files.locks.is_wanted = true;
            start(state)
        }
        LargeFilesIntent::ReadLocksAfterOperation => {
            state.large_files.locks.after_operation = true;
            follow(state)
        }
    }
}

/// Opens the page of the repository shown, or shows the one there is; an opening reads the locks.
fn open(state: &mut State) -> Vec<Effect> {
    if state.shown_folder().is_none() {
        return Vec::new();
    }
    if !state.pages.open.contains(&Page::LargeFiles) {
        state.large_files.locks.is_wanted = true;
    }
    super::pages::show_page(state, Page::LargeFiles);
    follow(state)
}

/// Reads the files while the page is open and the repository shown was read since, or is another one, and the locks
/// when asked; cancels the read once the page is closed.
pub fn follow(state: &mut State) -> Vec<Effect> {
    if !state.pages.open.contains(&Page::LargeFiles) {
        return cancel(state);
    }
    let Some(read) = state.shown_read() else {
        return Vec::new();
    };
    let (folder, generation) = (read.state.folder.clone(), read.generation);
    let page = &state.large_files;
    if !page.folder.as_deref().is_some_and(|known| same_folder(known, &folder)) {
        let mut effects = cancel(state);
        state.large_files = LargeFilesPage {
            folder: Some(folder),
            locks: LockReads {
                is_wanted: true,
                ..LockReads::default()
            },
            ..LargeFilesPage::default()
        };
        effects.extend(start(state));
        return effects;
    }
    if page.locks.after_operation && state.operations.running.is_none() {
        state.large_files.locks.after_operation = false;
        state.large_files.locks.is_wanted = true;
    }
    let page = &state.large_files;
    if page.generation == Some(generation) && !(page.locks.is_wanted && page.job.is_none()) {
        return Vec::new();
    }
    start(state)
}

/// Starts a read of the files, with the locks when asked, or asks for one after the read running.
fn start(state: &mut State) -> Vec<Effect> {
    let Some(generation) = state.shown_read().map(|read| read.generation) else {
        return Vec::new();
    };
    let page = &mut state.large_files;
    let Some(folder) = page.folder.clone() else {
        return Vec::new();
    };
    page.generation = Some(generation);
    if page.job.is_some() {
        page.is_read_again = true;
        return Vec::new();
    }
    let with_locks = std::mem::take(&mut page.locks.is_wanted);
    page.locks.is_reading = with_locks;
    let job = super::workspace::start_job(state, JobKind::ReadLargeFiles(folder, with_locks));
    state.large_files.job = Some(job.id);
    vec![Effect::Start(job)]
}

/// Cancels the read running once the page is closed; the next opening reads again.
fn cancel(state: &mut State) -> Vec<Effect> {
    let page = &mut state.large_files;
    page.folder = None;
    page.locks.is_reading = false;
    let Some(job) = page.job.take() else {
        return Vec::new();
    };
    let _cancelled = state.jobs.remove(&job);
    vec![Effect::Cancel(job)]
}

/// Takes what a read gave, keeping the locks read before when it did not ask for them, and reads again when asked
/// meanwhile.
pub fn read(state: &mut State, job: JobId, result: Result<LargeFilesRead, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let page = &mut state.large_files;
    if page.job != Some(job) {
        return Vec::new();
    }
    page.job = None;
    page.locks.is_reading = false;
    let before = page.read.take().and_then(Result::ok).and_then(|read| read.locks);
    page.read = Some(result.map(|mut read| {
        if read.locks.is_none() {
            read.locks = before;
        }
        read
    }));
    if std::mem::take(&mut page.is_read_again) || page.locks.is_wanted {
        return start(state);
    }
    Vec::new()
}

/// What the page shows.
pub fn page_state(state: &State) -> LfsPageState<'_> {
    let page = &state.large_files;
    let read = page.read.as_ref().and_then(|read| read.as_ref().ok());
    let locks = read.and_then(|read| read.locks.as_ref());
    LfsPageState {
        status: read.map(|read| &read.status),
        files: read.map_or(&[], |read| read.files.as_slice()),
        is_reading: page.job.is_some(),
        failed: page
            .read
            .as_ref()
            .and_then(|read| read.as_ref().err())
            .map(String::as_str),
        locks: locks.and_then(|locks| locks.as_ref().ok()),
        is_reading_locks: page.locks.is_reading,
        locks_failed: locks.and_then(|locks| locks.as_ref().err()).map(String::as_str),
        remote: state.shown_read().and_then(|read| default_remote(&read.state.remotes)),
    }
}

/// The remote Git LFS asks for the locks, as `remote.default` chooses it: `origin`, else the first.
fn default_remote(remotes: &[String]) -> Option<&str> {
    remotes
        .iter()
        .find(|remote| *remote == "origin")
        .or_else(|| remotes.first())
        .map(String::as_str)
}
