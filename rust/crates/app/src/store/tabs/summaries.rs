//! What the repositories open and their worktrees and submodules are on, for the rows of **Repositories** and the
//! entries of their **▾** (`repo.worktree-summary`): the folder shown says it from its own read; every other one is
//! read in the background once the window opens, after the read of the folder shown, then again only when its files
//! or its git folder change, one batch at a time and never while an operation runs.

use std::path::{Path, PathBuf};
use std::time::Instant;

use model::repo::{Change, group_tabs};
use views::repo::TabSummary;

use super::super::message::Effect;
use super::super::state::State;
use super::menu::TabShows;
use crate::persisted::same_folder;
use crate::work::summaries::{GitFolderStamp, SummaryAsk, SummaryRead};

/// What is known of the folders read in the background.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Summaries {
    /// Each folder, in the order the rows list them.
    pub entries: Vec<SummaryEntry>,
    /// Whether a batch of reads is under way.
    pub is_reading: bool,
}

/// A folder read in the background.
#[derive(Clone, Debug, PartialEq)]
pub struct SummaryEntry {
    /// The folder of the repository, the worktree or the submodule.
    pub folder: PathBuf,
    /// What it said last; none before its first read.
    pub summary: Option<TabSummary>,
    /// Its git folder at its last read.
    pub stamp: Option<GitFolderStamp>,
    /// When it was last read.
    pub read_at: Option<Instant>,
    /// Whether it is to be read: never read, or something changed in it since.
    pub is_due: bool,
    /// Whether files of its working tree changed, which reads it whatever its git folder says.
    pub is_forced: bool,
}

impl Summaries {
    /// What a folder said last, read in the background.
    #[must_use]
    pub fn of(&self, folder: &Path) -> Option<&TabSummary> {
        self.entries
            .iter()
            .find(|entry| same_folder(&entry.folder, folder))
            .and_then(|entry| entry.summary.as_ref())
    }

    /// The folders read in the background, for their watchers.
    #[must_use]
    pub fn folders(&self) -> Vec<PathBuf> {
        self.entries.iter().map(|entry| entry.folder.clone()).collect()
    }
}

/// Every folder whose row or entry says what it is on, the folder shown left out: the folder each tab shows, and every
/// repository of its group, each once.
fn wanted(state: &State) -> Vec<PathBuf> {
    let groups = group_tabs(state.repositories());
    let shown_folder = state.shown_folder();
    let mut folders: Vec<PathBuf> = Vec::new();
    for (index, tab) in state.tabs.open.iter().enumerate() {
        let tab_shows = TabShows::new(state, (index, tab), &groups, "");
        let members = tab_shows.members.iter().map(|member| member.folder.clone());
        for folder in std::iter::once(tab_shows.folder.clone()).chain(members) {
            let is_shown = shown_folder.is_some_and(|shown| same_folder(shown, &folder));
            if !is_shown && !folders.iter().any(|known| same_folder(known, &folder)) {
                folders.push(folder);
            }
        }
    }
    folders
}

/// After each message: the folders known follow the tabs and their groups, and the folders due are read in one batch,
/// once the folder shown is read and nothing else runs.
pub fn follow(state: &mut State) -> Option<Effect> {
    if state.workspace.is_none() {
        state.tabs.summaries = Summaries::default();
        return None;
    }
    let folders = wanted(state);
    let summaries = &mut state.tabs.summaries;
    summaries
        .entries
        .retain(|entry| folders.iter().any(|folder| same_folder(folder, &entry.folder)));
    for folder in folders {
        if !summaries
            .entries
            .iter()
            .any(|entry| same_folder(&entry.folder, &folder))
        {
            summaries.entries.push(SummaryEntry {
                folder,
                summary: None,
                stamp: None,
                read_at: None,
                is_due: true,
                is_forced: true,
            });
        }
    }
    let is_busy = state.is_operation_running
        || state.opening.is_some()
        || state.reads.schedule.is_reading()
        || state.shown_read().is_none();
    let summaries = &mut state.tabs.summaries;
    if is_busy || summaries.is_reading {
        return None;
    }
    let asks: Vec<SummaryAsk> = summaries
        .entries
        .iter_mut()
        .filter(|entry| entry.is_due)
        .map(|entry| {
            entry.is_due = false;
            SummaryAsk {
                folder: entry.folder.clone(),
                last: entry.stamp.clone(),
                read_at: entry.read_at,
                is_forced: std::mem::take(&mut entry.is_forced),
            }
        })
        .collect();
    summaries.is_reading = !asks.is_empty();
    (!asks.is_empty()).then_some(Effect::ReadSummaries(asks))
}

/// What a batch of reads found: a summary read takes the place of the one before; a read that failed keeps it, muted;
/// a folder in which nothing changed keeps it as it was.
pub fn read(state: &mut State, reads: Vec<SummaryRead>) {
    let summaries = &mut state.tabs.summaries;
    summaries.is_reading = false;
    for read in reads {
        let Some(entry) = summaries
            .entries
            .iter_mut()
            .find(|entry| same_folder(&entry.folder, &read.folder))
        else {
            continue;
        };
        entry.stamp = read.stamp;
        entry.read_at = Some(read.read_at);
        match read.result {
            Some(Ok(summary)) => {
                entry.summary = Some(TabSummary {
                    summary,
                    is_stale: false,
                });
            }
            Some(Err(_)) => {
                if let Some(summary) = entry.summary.as_mut() {
                    summary.is_stale = true;
                }
            }
            None => {}
        }
    }
}

/// Changes seen in a folder read in the background mark it due: a file of its working tree reads it whatever its git
/// folder says; a change of its git folder reads it only when what [`crate::work::summaries::git_folder_changed`]
/// looks at changed.
pub fn changed(state: &mut State, folder: &Path, changes: &[Change]) {
    let Some(entry) = state
        .tabs
        .summaries
        .entries
        .iter_mut()
        .find(|entry| same_folder(&entry.folder, folder))
    else {
        return;
    };
    let in_files = changes
        .iter()
        .any(|change| matches!(change, Change::WorkingTree(_) | Change::Saved(_) | Change::Files(_)));
    let in_git = changes.iter().any(|change| matches!(change, Change::GitFolder(_)));
    if in_files || in_git {
        entry.is_due = true;
        entry.is_forced |= in_files;
    }
}
