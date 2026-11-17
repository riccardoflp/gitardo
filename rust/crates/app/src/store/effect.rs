//! What the store of a window asks of the outside ([`Effect`]), carried out after each reduce.

use std::path::PathBuf;
use std::time::Duration;

use super::message::{Job, JobId};
use crate::persisted::Persisted;
use crate::settings::SettingsFiles;

/// What the store asks of the outside, carried out after the reduce, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// Runs a job on a worker.
    Start(Job),
    /// Cancels a job that is not wanted any more: the git it runs is stopped.
    Cancel(JobId),
    /// Asks the AI provider a question, on a thread of its own (`ai.resolve-conflicts`, `commit.draft-message`).
    Ask(Box<crate::work::ai::AiQuestion>),
    /// Cancels questions to the AI provider under way: nothing more comes of them.
    StopAsking(Vec<crate::work::ai::AskId>),
    /// Opens the system's folder dialog, whose answer comes back as [`Message::FolderChosen`].
    ChooseFolder,
    /// Opens the system's dialog for a workspace file of VS Code, whose answer comes back as
    /// [`Message::WorkspaceFileChosen`].
    ChooseWorkspaceFile,
    /// Writes what is kept between runs.
    WritePersisted(Persisted),
    /// Sets a value in a settings file, keeping the rest of its text, on a thread of its own: a switch of the timeline
    /// in the user's file.
    WriteSetting {
        /// The file.
        file: PathBuf,
        /// The key.
        key: &'static str,
        /// The value.
        value: json::Value,
    },
    /// Watches these settings files, in place of those watched before.
    WatchSettings(SettingsFiles),
    /// Draws the window with the theme and the fonts of the settings in force.
    ApplySettings,
    /// Watches this working tree and its git folder for changes, in place of the one watched before; none stops
    /// watching (`repo.watch-changes`).
    WatchRepository(Option<PathBuf>),
    /// Watches these working trees and their git folders, those of the tabs not shown, in place of the ones watched
    /// before; each sends [`Message::TabChanged`] (`repo.watch-hidden`).
    WatchTabs(Vec<PathBuf>),
    /// Reads what these folders are on, on a thread of its own, which sends [`Message::SummariesRead`]
    /// (`repo.worktree-summary`).
    ReadSummaries(Vec<crate::work::summaries::SummaryAsk>),
    /// Sends [`Message::Wake`] at this moment, counted from when the window opened, in place of the wake asked for
    /// before (`repo.schedule-reads`).
    WakeAt(Duration),
    /// Runs an operation of the catalog in a repository, on the thread of the operations; the window sets the time of
    /// its context.
    RunFeature {
        /// The repository.
        folder: PathBuf,
        /// The operation.
        feature: model::contract::FeatureId,
        /// The answers it is given before it asks: none, or what the entry chosen names.
        preset: model::contract::Answers,
        /// What it is given, boxed for its settings.
        context: Box<model::contract::Context>,
    },
    /// Gives the operation running what the user answered.
    ReplyToFeature(super::feature::Reply),
    /// Puts text on the clipboard of the system: the URL of a remote (`remote.manage`).
    CopyText(String),
    /// Opens a folder in a new window of Gitardo Desktop, as a repository of its own
    /// (`submodule.open-in-new-window`, `worktree.open-in-new-window`).
    OpenWindow(PathBuf),
    /// Opens the system's dialog to choose a file to open, or where to save one, for the question of the operation
    /// running; the path chosen, or none, is its answer.
    ChooseFile {
        /// The title of the dialog.
        title: String,
        /// Where it starts and which files it shows.
        question: model::contract::FileQuestion,
        /// Whether it chooses where to save, rather than a file to open.
        is_saving: bool,
    },
    /// Looks for the repository on GitHub or reads its pull requests, on a thread of its own (`github.read-pull-requests`).
    GitHub(super::github::GitHubJob),
    /// Runs, answers or stops the tasks of the Activity drawer (`worktree-setup.command-steps`).
    Tasks(super::tasks::TaskEffect),
    /// Starts, writes to, sizes or ends the terminals of the bottom panel (`terminal.view`).
    Terminal(super::terminals::TerminalEffect),
    /// Opens an address in the browser of the system: a pull request on GitHub (`github.open-pull-request`).
    OpenUrl(String),
    /// Minimizes, maximizes, restores or closes the window, as a button of its title bar asked
    /// (`window.caption-buttons`).
    Window(views::window::WindowIntent),
    /// Closes the window and ends the program.
    Quit,
}
