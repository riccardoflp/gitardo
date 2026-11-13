//! The store of a window (`docs/02-architecture.md`, State): what the window shows, changed only by [`reduce`], which
//! takes a [`Message`] and gives the [`Effect`]s the outside carries out after it: jobs on the workers, the folder
//! dialog, the file of what is kept between runs, the settings files to watch.
//!
//! `reduce` reads no file and runs no git: it is tested without a window.

pub mod blame;
mod changes;
mod commit_page;
pub mod compare;
mod conflicts;
mod diff_page;
pub mod feature;
pub mod file_page;
pub mod files_page;
mod inspector;
mod log_lines;
mod merge_page;
mod message;
mod pages;
mod rail;
mod reads;
mod rebase;
pub mod review;
pub mod search;
mod settings_page;
mod stashes;
mod state;
#[cfg(test)]
mod tests;
mod timeline;
mod view_parts;
mod view_state;
mod workspace;
mod worktrees;

use std::path::PathBuf;

use views::{FolderAnswer, Intent};

use crate::persisted;
use crate::settings::Settings;

pub use changes::{Changes, ChangesData};
pub use commit_page::{CommitPage, CommitPages, PageReadState};
pub use diff_page::{DiffPage, DiffPages};
pub use file_page::{FileContent, FilePage, FilePages};
pub use inspector::{Inspector, InspectorRead, Subject, SubjectRow};
pub use merge_page::{MergePage, MergePages};
pub use message::{Effect, Found, Job, JobId, JobKind, Known, Message, MorePages, TimelineRequest};
pub use rail::{FilterPicker, Rail};
pub use reads::Reads;
pub use stashes::StashesList;
pub use state::{Dialog, Opening, Pages, RepositoryRead, Shown, State, Workspace};
pub use timeline::{Timeline, commits_request, draft_of, history_of, known};
pub use view_state::ViewData;
pub use workspace::OpenRequest;
pub use worktrees::Worktrees;

/// Changes the store as a message says, and gives what the outside is to do after it, in order: then the inspector
/// follows the row selected, the pages closed are forgotten, **Show in the timeline** follows the timeline read, and
/// the repository shown is watched, when it is not the one watched.
pub fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    let mut effects = reduce_message(state, message);
    effects.extend(inspector::follow(state));
    effects.extend(commit_page::prune(state));
    effects.extend(commit_page::follow(state));
    effects.extend(diff_page::prune(state));
    effects.extend(diff_page::follow(state));
    effects.extend(merge_page::prune(state));
    effects.extend(merge_page::follow(state));
    effects.extend(file_page::prune(state));
    effects.extend(file_page::follow(state));
    effects.extend(compare::prune(state));
    effects.extend(compare::follow(state));
    effects.extend(review::prune(state));
    effects.extend(review::follow(state));
    effects.extend(files_page::follow(state));
    effects.extend(blame::follow(state));
    effects.extend(reads::watch(state));
    effects
}

/// Changes the store as a message says.
fn reduce_message(state: &mut State, message: Message) -> Vec<Effect> {
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
        } => repository_read(state, job, &folder, (result, read_at)),
        Message::FeatureAsks(question) => changes::asks(state, *question),
        Message::FeatureConfirms(plan) => {
            feature::confirms(state, *plan);
            Vec::new()
        }
        Message::FeatureEnded { ended, log } => {
            changes::ended(state, &ended);
            file_page::ended(state, &ended);
            if let feature::Ended::Ran { after, .. } = &ended {
                rebase::ended(state, after);
            }
            feature::ended(state, ended, log)
        }
        Message::CommitsRead { job, result } => timeline::commits_read(state, job, result),
        Message::TimelineRead { job, result } => timeline::timeline_read(state, job, result),
        Message::Searched { job, result } => search::searched(state, job, result),
        Message::DetailsRead { job, result } if stashes::is_files_read(state, job) => {
            stashes::files_read(state, job, result)
        }
        Message::DetailsRead { job, result } => inspector::details_read(state, job, result),
        Message::CommitPageRead { job, result } => commit_page::page_read(state, job, result),
        Message::DiffRead { job, result } => diff_page::diff_read(state, job, result),
        Message::MergeRead { job, result } => merge_page::merge_read(state, job, result),
        Message::FileRead { job, result } => file_page::file_read(state, job, result),
        Message::ComparisonRead { job, result } => compare::comparison_read(state, job, result),
        Message::ReviewRead { job, result } => review::review_read(state, job, result),
        Message::InPlaceRead { job, result } => compare::in_place::in_place_read(state, job, result),
        Message::ConflictsCounted { job, result } => conflicts::counted(state, job, result),
        Message::FilesRead { job, result } => files_page::files_read(state, job, result),
        Message::BlameRead { job, result } => blame::blame_read(state, job, result),
        Message::SettingsRead(settings) => settings_read(state, *settings),
        Message::Changed { changes, at } => reads::changed(state, &changes, at),
        Message::Wake { at } => reads::wake(state, at),
        Message::Minimized(is_minimized) => reads::minimized(state, is_minimized),
        Message::ReadTook { line, took, time } => {
            reads::read_took(state, (line, took, time));
            Vec::new()
        }
        Message::CloseRequested { window_size } => {
            let mut effects = workspace::cancel_all(state);
            state.persisted.window_size = window_size.or(state.persisted.window_size);
            effects.push(Effect::WritePersisted(state.persisted.clone()));
            effects.push(Effect::Quit);
            effects
        }
    }
}

/// A read of the repository ended: shown, unless the schedule throws it away; then the read that follows it, if one
/// does. At `log.level` `debug`, what git said of a read that failed goes to the Command log.
fn repository_read(
    state: &mut State,
    job: JobId,
    folder: &std::path::Path,
    (result, read_at): (ReadResult, i64),
) -> Vec<Effect> {
    let (is_shown, follow) = reads::finished(state, job);
    if let (Err(error), engine::LogLevel::Debug) = (&result, state.log.level()) {
        state.log.record_failure(model::Timestamp(read_at), error);
    }
    let mut effects = if is_shown {
        let mut effects = workspace::repository_read(state, job, folder, (result, read_at));
        effects.extend(feature::after_read(state));
        effects.extend(conflicts::after_read(state));
        effects
    } else {
        let _thrown_away = state.jobs.remove(&job);
        Vec::new()
    };
    effects.extend(follow);
    effects
}

/// What a read of a repository found, or what git said.
type ReadResult = Result<Box<(model::state::RepositoryState, git::read::graph::CommitPage)>, String>;

/// The settings read again: the theme and the fonts drawn with, the switches of the timeline in force, the level of the
/// Command log; a change to a setting that changes what is read reads the repository again.
fn settings_read(state: &mut State, settings: Settings) -> Vec<Effect> {
    let looks_different = settings.theme != state.settings.theme || settings.fonts != state.settings.fonts;
    let before = state.settings.switches;
    let sorted_before = state.settings.sort_order;
    let changes_reads = settings_page::changes_reads(&state.settings, &settings);
    state.settings = settings;
    state.log.set_level(state.settings.log_level);
    if state.settings.sort_order != sorted_before {
        rail::arrange(state);
    }
    let mut effects = Vec::new();
    if looks_different {
        effects.push(Effect::ApplySettings);
    }
    for switch in views::graph::TimelineSwitch::ALL {
        if before.is_on(switch) != state.settings.switches.is_on(switch) {
            effects.extend(timeline::apply_switch(state, switch));
        }
    }
    if changes_reads && !effects.iter().any(|effect| matches!(effect, Effect::Start(_))) {
        effects.extend(reads::asked(state, model::repo::ReadCause::Settings));
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
    if !folders.is_empty() && !state.settings.read_on_startup {
        state.workspace = Some(Workspace {
            folders,
            repositories: None,
            shown: Shown::Looking,
        });
        state.reads.is_not_read = true;
    } else if !folders.is_empty() {
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
        Intent::ReadRepository => {
            state.reads.is_not_read = false;
            look_again(state, false)
        }
        Intent::CheckAgain | Intent::Refresh if state.reads.is_not_read => {
            state.reads.is_not_read = false;
            look_again(state, false)
        }
        Intent::Refresh if state.reads.schedule.is_reading() => reads::asked(state, model::repo::ReadCause::Refresh),
        Intent::CheckAgain | Intent::Refresh => look_again(state, intent == Intent::Refresh),
        Intent::PickTheme | Intent::CloseThemePicker | Intent::SetSetting(_) | Intent::ChooseSettingsFile(_) => {
            settings_page::reduce_intent(state, &intent)
        }
        Intent::ShowTimeline => pages::show_timeline(state),
        Intent::ShowPage(page) => {
            pages::show_page(state, page);
            Vec::new()
        }
        Intent::NextPage => pages::next_page(state),
        Intent::ClosePage => file_page::close_page(state),
        Intent::ClosePicker
        | Intent::OpenActivity
        | Intent::CloseActivity
        | Intent::ShowCommandLog
        | Intent::HideCommandLog => {
            reduce_view_intent(state, &intent);
            Vec::new()
        }
        Intent::DismissNotice => {
            state.notice = None;
            Vec::new()
        }
        Intent::Rail(intent) => rail::reduce_rail(state, intent),
        Intent::Filter(intent) => rail::reduce_filter(state, intent),
        Intent::Graph(intent) => reduce_graph(state, intent),
        Intent::Search(intent) => search::reduce(state, intent),
        Intent::Details(intent) => inspector::reduce_details(state, intent),
        Intent::CommitPage(id, intent) => commit_page::reduce_page(state, id, intent),
        Intent::Diff(intent) => diff_page::reduce(state, intent),
        Intent::MergeView(intent) => merge_page::reduce(state, intent),
        Intent::FileView(_) | Intent::Files(_) | Intent::Blame(_) => files_page::reduce_file_intent(state, intent),
        Intent::Compare(intent) => compare::reduce(state, intent),
        Intent::Review(intent) => review::reduce(state, intent),
        Intent::Feature(intent) => feature::reduce(state, intent),
        Intent::Changes(intent) => changes::reduce(state, intent),
        Intent::Rebase(intent) => rebase::reduce(state, intent),
        Intent::Conflicts(intent) => conflicts::reduce(state, intent),
    }
}

/// An intent of the timeline; selecting a row there closes the comparison shown in place in the inspector.
fn reduce_graph(state: &mut State, intent: views::graph::GraphIntent) -> Vec<Effect> {
    if matches!(intent, views::graph::GraphIntent::Select(_)) {
        state.compares.in_place = None;
    }
    timeline::reduce_graph(state, intent)
}

/// An intent that changes only what a view shows.
fn reduce_view_intent(state: &mut State, intent: &Intent) {
    match intent {
        Intent::ClosePicker => state.dialog = None,
        Intent::OpenActivity => state.is_activity_open = true,
        Intent::CloseActivity => state.is_activity_open = false,
        Intent::ShowCommandLog => state.is_log_shown = true,
        Intent::HideCommandLog => state.is_log_shown = false,
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
