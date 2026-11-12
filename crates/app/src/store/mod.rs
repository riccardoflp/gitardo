//! The store of a window (`docs/02-architecture.md`, State): what the window shows, changed only by [`reduce`], which
//! takes a [`Message`] and gives the [`Effect`]s the outside carries out after it: jobs on the workers, the folder
//! dialog, the file of what is kept between runs, the settings files to watch.
//!
//! `reduce` reads no file and runs no git: it is tested without a window.

mod message;
mod pages;
mod state;
#[cfg(test)]
mod tests;
mod timeline;
mod view_state;
mod workspace;

use std::path::PathBuf;

use views::{FolderAnswer, Intent};

use crate::persisted;
use crate::settings::Settings;

pub use message::{Effect, Found, Job, JobId, JobKind, Known, Message, MorePages, TimelineRequest};
pub use state::{Dialog, Opening, Pages, RepositoryRead, Shown, State, Workspace};
pub use timeline::{Timeline, commits_request, draft_of, history_of, known};
pub use view_state::ViewData;
pub use workspace::OpenRequest;

/// Changes the store as a message says, and gives what the outside is to do after it, in order.
pub fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::Started { folders } => started(state, folders),
        Message::Intent(intent) => reduce_intent(state, intent),
        Message::FolderChosen(folder) => folder_chosen(state, folder),
        Message::RepositoriesFound { job, found } => workspace::repositories_found(state, job, found),
        Message::RepositoryRead {
            job,
            folder,
            result,
            read_at,
        } => workspace::repository_read(state, job, &folder, (result, read_at)),
        Message::CommitsRead { job, result } => timeline::commits_read(state, job, result),
        Message::SettingsRead(settings) => settings_read(state, settings),
        Message::CloseRequested { window_size } => {
            let mut effects = workspace::cancel_all(state);
            state.persisted.window_size = window_size.or(state.persisted.window_size);
            effects.push(Effect::WritePersisted(state.persisted.clone()));
            effects.push(Effect::Quit);
            effects
        }
    }
}

/// The settings read again: the theme and the fonts drawn with, the switches of the timeline in force.
fn settings_read(state: &mut State, settings: Settings) -> Vec<Effect> {
    let looks_different = settings.theme != state.settings.theme || settings.fonts != state.settings.fonts;
    let before = state.settings.switches;
    state.settings = settings;
    let mut effects = Vec::new();
    if looks_different {
        effects.push(Effect::ApplySettings);
    }
    for switch in views::graph::TimelineSwitch::ALL {
        if before.is_on(switch) != state.settings.switches.is_on(switch) {
            effects.extend(timeline::apply_switch(state, switch));
        }
    }
    effects
}

/// The window opened: the Welcome page the first time, and the workspace of the command line when it names one.
fn started(state: &mut State, folders: Vec<PathBuf>) -> Vec<Effect> {
    let mut effects = vec![workspace::watch_settings(state)];
    if !state.persisted.has_started {
        state.persisted.has_started = true;
        pages::show_page(state, views::page::Page::Welcome);
        effects.push(Effect::WritePersisted(state.persisted.clone()));
    }
    if !folders.is_empty() {
        let request = OpenRequest {
            is_shown_at_once: true,
            ..OpenRequest::default()
        };
        let was_welcome = state.pages.shown;
        effects.extend(workspace::open_workspace(state, folders, request));
        state.pages.shown = was_welcome;
    }
    effects
}

/// A folder chosen in the system's dialog: shown at once in a window with no workspace, asked about otherwise.
fn folder_chosen(state: &mut State, folder: Option<PathBuf>) -> Vec<Effect> {
    let Some(folder) = folder else {
        return Vec::new();
    };
    if state.workspace.is_some() {
        state.dialog = Some(Dialog::OpenFolder { folder });
        return Vec::new();
    }
    let request = OpenRequest {
        is_shown_at_once: true,
        ..OpenRequest::default()
    };
    workspace::open_workspace(state, vec![folder], request)
}

/// The answer to **Replace the workspace** or **Add to the workspace**: an added folder keeps the repository shown.
fn answer_open_folder(state: &mut State, answer: FolderAnswer) -> Vec<Effect> {
    let Some(Dialog::OpenFolder { folder }) = state.dialog.take() else {
        return Vec::new();
    };
    match answer {
        FolderAnswer::Cancel => Vec::new(),
        FolderAnswer::Replace => {
            let request = OpenRequest {
                is_shown_at_once: true,
                ..OpenRequest::default()
            };
            workspace::open_workspace(state, vec![folder], request)
        }
        FolderAnswer::Add => {
            let mut folders = state
                .workspace
                .as_ref()
                .map(|workspace| workspace.folders.clone())
                .unwrap_or_default();
            if !folders.iter().any(|known| persisted::same_folder(known, &folder)) {
                folders.push(folder);
            }
            let request = OpenRequest {
                remembered: state.shown_folder().cloned(),
                ..OpenRequest::default()
            };
            workspace::open_workspace(state, folders, request)
        }
    }
}

/// What a view returned.
fn reduce_intent(state: &mut State, intent: Intent) -> Vec<Effect> {
    match intent {
        Intent::OpenFolder => {
            state.dialog = None;
            vec![Effect::ChooseFolder]
        }
        Intent::AnswerOpenFolder(answer) => answer_open_folder(state, answer),
        Intent::OpenRecent(index) => open_recent(state, index),
        Intent::SelectRepository(folder) => workspace::select(state, &folder),
        Intent::PickRepository => {
            workspace::pick_repository(state);
            Vec::new()
        }
        Intent::CheckAgain | Intent::Refresh => look_again(state, intent == Intent::Refresh),
        Intent::ShowTimeline => pages::show_timeline(state),
        Intent::ShowPage(page) => {
            pages::show_page(state, page);
            Vec::new()
        }
        Intent::NextPage => pages::next_page(state),
        Intent::ClosePage => pages::close_page(state),
        Intent::ClosePicker | Intent::OpenActivity | Intent::CloseActivity | Intent::ToggleKept(_) => {
            reduce_view_intent(state, &intent);
            Vec::new()
        }
        Intent::DismissNotice => {
            state.notice = None;
            Vec::new()
        }
        Intent::Graph(intent) => timeline::reduce_graph(state, intent),
    }
}

/// An intent that changes only what a view shows.
fn reduce_view_intent(state: &mut State, intent: &Intent) {
    match intent {
        Intent::ClosePicker => state.dialog = None,
        Intent::OpenActivity => state.is_activity_open = true,
        Intent::CloseActivity => state.is_activity_open = false,
        Intent::ToggleKept(list) => {
            if let Some(index) = state.kept_open.iter().position(|open| open == list) {
                let _closed = state.kept_open.remove(index);
            } else {
                state.kept_open.push(*list);
            }
        }
        _ => {}
    }
}

/// Opens a recent workspace, with the repository shown in it last; the window keeps what it shows until its
/// repositories are found, since one whose folders are all gone is refused.
fn open_recent(state: &mut State, index: usize) -> Vec<Effect> {
    let Some(recent) = state.persisted.recent.get(index).cloned() else {
        return Vec::new();
    };
    let request = OpenRequest {
        remembered: recent.repository,
        recent: Some(index),
        ..OpenRequest::default()
    };
    workspace::open_workspace(state, recent.folders, request)
}

/// **Check again** looks for the repositories of the workspace again and shows the first one, saying **Loading
/// repository…** meanwhile; **Refresh** reads again the repository shown, keeping it on the screen.
fn look_again(state: &mut State, is_refresh: bool) -> Vec<Effect> {
    let Some(folders) = state.workspace.as_ref().map(|workspace| workspace.folders.clone()) else {
        return Vec::new();
    };
    let request = OpenRequest {
        remembered: if is_refresh {
            state.shown_folder().cloned()
        } else {
            None
        },
        recent: None,
        is_shown_at_once: !is_refresh,
        is_refresh,
    };
    workspace::open_workspace(state, folders, request)
}
