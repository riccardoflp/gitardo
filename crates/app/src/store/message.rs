//! What the store of a window is told ([`Message`]) and what it asks of the outside ([`Effect`]).

use std::path::PathBuf;

use git::read::graph::CommitPage;
use model::graph::PageSize;
use model::repo::Repository;
use model::state::RepositoryState;
use views::Intent;

use crate::persisted::Persisted;
use crate::settings::{Settings, SettingsFiles};

/// What the store of a window is told: what the user wants, and what the outside answers.
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// The window opened, on the folders of the command line, or on none.
    Started {
        /// The folders of the command line: `gitardo <folder>...`.
        folders: Vec<PathBuf>,
    },
    /// What a view returned.
    Intent(Intent),
    /// The answer of the system's folder dialog: none when it was cancelled.
    FolderChosen(Option<PathBuf>),
    /// A job that looked for the repositories of folders is done.
    RepositoriesFound {
        /// The job.
        job: JobId,
        /// What it found.
        found: Found,
    },
    /// A job that read a repository is done.
    RepositoryRead {
        /// The job.
        job: JobId,
        /// The folder read.
        folder: PathBuf,
        /// What was read, or what git said when the read failed.
        result: Result<Box<(RepositoryState, CommitPage)>, String>,
    },
    /// The settings files changed and were read again.
    SettingsRead(Settings),
    /// The person asked to close the window, whose inside had this size in logical pixels.
    CloseRequested {
        /// The size of the inside of the window.
        window_size: Option<(f32, f32)>,
    },
}

/// The repositories found in some folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    /// The repositories, in the order they were found; none when the folders hold none.
    Repositories(Vec<Repository>),
    /// None of the folders is there any more.
    AllGone,
    /// Git could not be run: what it said.
    Failed(String),
}

/// The number of a job, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(pub u64);

/// What a job does, on a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobKind {
    /// Looks for the repositories of folders (`repo.read-repositories`).
    FindRepositories {
        /// The folders.
        folders: Vec<PathBuf>,
    },
    /// Reads a repository: its state (`repo.read-state`), then the first page of its commits (`graph.read-commits`).
    ReadRepository {
        /// The folder of the repository.
        folder: PathBuf,
        /// How many commits a page has.
        page_size: PageSize,
    },
}

/// A job for a worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// Its number, which its result carries back.
    pub id: JobId,
    /// What it does.
    pub kind: JobKind,
}

/// What the store asks of the outside, carried out after the reduce, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Runs a job on a worker.
    Start(Job),
    /// Cancels a job that is not wanted any more: the git it runs is stopped.
    Cancel(JobId),
    /// Opens the system's folder dialog, whose answer comes back as [`Message::FolderChosen`].
    ChooseFolder,
    /// Writes what is kept between runs.
    WritePersisted(Persisted),
    /// Watches these settings files, in place of those watched before.
    WatchSettings(SettingsFiles),
    /// Draws the window with the theme and the fonts of the settings in force.
    ApplySettings,
    /// Closes the window and ends the program.
    Quit,
}
