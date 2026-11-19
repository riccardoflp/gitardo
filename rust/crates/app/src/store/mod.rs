//! The store of a window (`docs/desktop/02-architecture.md`, State): what the window shows, changed only by
//! [`reduce`], which takes a [`Message`] and gives the [`Effect`]s the outside carries out after it: jobs on the
//! workers, the folder dialog, the file of what is kept between runs, the settings files to watch.
//!
//! `reduce` reads no file and runs no git: it is tested without a window.

pub mod blame;
mod changed_files;
mod changes;
mod changes_beside;
mod co_authors;
mod code_workspace;
mod commit_options;
mod commit_page;
pub mod compare;
mod conflicts;
mod diff_page;
mod draft;
mod effect;
pub mod feature;
pub mod file_page;
pub mod file_search;
pub mod files_page;
pub mod github;
mod inspector;
mod job;
mod log_lines;
mod merge_page;
mod message;
pub mod new_worktrees;
mod pages;
pub mod panel;
mod rail;
pub mod range_diff;
mod reads;
mod rebase;
pub mod review;
pub mod search;
mod settings_page;
mod signature;
mod stashes;
mod state;
pub mod tabs;
pub mod tasks_tab;
pub mod terminals;
#[cfg(test)]
mod tests;
mod timeline;
mod timeline_message;
mod trust;
mod view_parts;
mod view_state;
mod workspace;
mod worktrees;

use std::path::PathBuf;

use views::{FolderAnswer, Intent};

use crate::persisted;
use crate::settings::Settings;

pub use changes::{Changes, ChangesData};
pub use code_workspace::code_workspace_folders;
pub use commit_page::{CommitPage, CommitPages, PageReadState};
pub use diff_page::{DiffPage, DiffPages};
pub use effect::Effect;
pub use file_page::{FileContent, FilePage, FilePages};
pub use inspector::{Inspector, InspectorRead, Subject, SubjectRow};
pub use merge_page::{MergePage, MergePages};
pub use message::{Found, Job, JobId, JobKind, Known, Message, MorePages, TimelineRequest};
pub use rail::{FilterPicker, Rail};
pub use reads::Reads;
pub use signature::SignatureCheck;
pub use stashes::StashesList;
pub use state::{Dialog, Opened, Opening, Pages, RepositoryRead, Shown, State, Workspace};
pub use timeline::{Timeline, commits_request, draft_of, history_of, known};
pub use timeline_message::job_done;
pub use view_state::ViewData;
pub use workspace::OpenRequest;
pub use worktrees::Worktrees;

/// Changes the store as a message says, and gives what the outside is to do after it, in order: then the inspector
/// follows the row selected, the pages closed are forgotten, **Show in the timeline** follows the timeline read, the
/// repository shown is watched, when it is not the one watched, and the tabs not shown are watched and read
/// (`repo.watch-hidden`). What the operation running says goes to its tab first (`repo.switch-tab`).
pub fn reduce(state: &mut State, message: Message) -> Vec<Effect> {
    let (mut effects, message) = tabs::before(state, message);
    let Some(message) = message else {
        effects.extend(tabs::follow_hidden(state));
        return effects;
    };
    if let Some(tab) = tabs::waiting_for(state, &message) {
        tabs::keep_for(state, tab, message);
        effects.extend(tabs::follow_hidden(state));
        return effects;
    }
    effects.extend(reduce_message(state, message));
    effects.extend(inspector::follow(state));
    effects.extend(commit_page::prune(state));
    effects.extend(commit_page::follow(state));
    effects.extend(changes_beside::follow(state));
    effects.extend(diff_page::prune(state));
    effects.extend(diff_page::follow(state));
    effects.extend(merge_page::prune(state));
    effects.extend(merge_page::follow(state));
    effects.extend(file_page::prune(state));
    effects.extend(file_page::follow(state));
    effects.extend(tabs::follow_closing(state));
    effects.extend(compare::prune(state));
    effects.extend(compare::follow(state));
    effects.extend(review::prune(state));
    effects.extend(review::follow(state));
    effects.extend(files_page::follow(state));
    effects.extend(file_search::follow(state));
    effects.extend(new_worktrees::follow(state));
    effects.extend(changes::follow_unsaved(state));
    effects.extend(blame::follow(state));
    effects.extend(reads::watch(state));
    effects.extend(terminals::follow(state));
    effects.extend(tasks_tab::follow(state));
    effects.extend(tabs::follow_hidden(state));
    effects
}

/// An operation ended: the pages that wait on it hear of it, then the store.
fn feature_ended(state: &mut State, ended: feature::Ended, log: Vec<engine::LogEntry>) -> Vec<Effect> {
    changes::ended(state, &ended);
    file_page::ended(state, &ended);
    if let feature::Ended::Ran { after, .. } = &ended {
        rebase::ended(state, after);
    }
    feature::ended(state, ended, log)
}

/// Changes the store as a message says.
pub(super) fn reduce_message(state: &mut State, message: Message) -> Vec<Effect> {
    match message {
        Message::Started { folders } => started(state, folders),
        Message::Intent(intent) => reduce_intent(state, intent),
        Message::FolderChosen(folder) => open_folder(state, folder),
        Message::WorkspaceFileChosen(chosen) => code_workspace::open_code_workspace(state, chosen),
        Message::RepositoriesFound { job, found } => tabs::repositories_found(state, job, found),
        Message::RepositoryRead {
            job,
            folder,
            result,
            read_at,
        } => repository_read(state, job, &folder, (result, read_at)),
        Message::FeatureAsks(question) => changes::asks(state, *question),
        Message::FeatureConfirms(plan) => feature::confirms(state, *plan),
        Message::FeatureEnded { ended, log } => feature_ended(state, ended, log),
        Message::CommitsRead { job, result } => timeline::commits_read(state, job, result),
        Message::PagesRead { job, result } => timeline::pages_read(state, job, result),
        Message::TimelineRead { job, result } => timeline::timeline_read(state, job, result),
        Message::Searched { job, result } => search::searched(state, job, result),
        Message::DetailsRead { job, result } if stashes::is_files_read(state, job) => {
            stashes::files_read(state, job, result)
        }
        Message::DetailsRead { job, result } => inspector::details_read(state, job, result),
        Message::CommitPageRead { job, result } => commit_page::page_read(state, job, result),
        Message::SignatureRead { job, result } => signature::signature_read(state, job, result),
        Message::AuthorsRead { job, result } => co_authors::authors_read(state, job, result),
        Message::DiffRead { job, result } => diff_page::diff_read(state, job, result),
        Message::MergeRead { job, result } => merge_page::merge_read(state, job, result),
        Message::FileRead { job, result } => file_page::file_read(state, job, result),
        message @ (Message::ImageRead { .. } | Message::DiagramDrawn { .. }) => file_page::preview_read(state, message),
        Message::ComparisonRead { job, result } => compare::comparison_read(state, job, result),
        Message::ReviewRead { job, result } => review::review_read(state, job, result),
        Message::InPlaceRead { job, result } => compare::in_place::in_place_read(state, job, result),
        Message::ConflictsCounted { job, result } => conflicts::counted(state, job, result),
        message @ (Message::FilesRead { .. } | Message::NewWorktreesRead { .. }) => files_page::read(state, message),
        Message::RangeDiffRead { job, result } => range_diff::pairs_read(state, job, result),
        Message::IgnoredRead { job, result } => reads::ignored_read(state, job, result),
        Message::FileSearch { job, answer } => file_search::answered(state, job, answer),
        Message::BlameRead { job, result } => blame::blame_read(state, job, result),
        Message::SettingsRead(settings) => settings_read(state, *settings),
        Message::Changed { changes, at } => reads::changed(state, &changes, at),
        Message::TabChanged { folder, changes, at } => tabs::tab_changed(state, &folder, &changes, at),
        Message::Wake { at } => reads::wake(state, at),
        message @ (Message::SummariesRead(_) | Message::ReadTook { .. }) => reads::noted(state, message),
        Message::Minimized(is_minimized) => reads::minimized(state, is_minimized),
        Message::Caption(caption) => workspace::caption_changed(state, caption),
        message @ (Message::AiPiece { .. } | Message::AiAnswered { .. }) => review::ai::reduce(state, message),
        Message::GitHub(message) => github::reduce(state, message),
        Message::Commands(message) => tasks_tab::reduce_commands(state, message),
        Message::Terminal(message) => terminals::reduce_message(state, message),
        Message::TaskRun(message) => tasks_tab::reduce_message(state, message),
        Message::UrlNotOpened(said) => github::not_opened(state, said),
        Message::CloseRequested(window) => tasks_tab::close_requested(state, window),
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
        effects.extend(github::after_read(state, read_at));
        effects
    } else {
        let _thrown_away = state.jobs.remove(&job);
        Vec::new()
    };
    effects.extend(follow);
    effects
}

/// What a read of a repository found, or what git said.
pub(super) type ReadResult = Result<
    Box<(
        model::state::RepositoryState,
        Option<git::read::graph::CommitPage>,
        git::read::worktree::CountedWorktrees,
    )>,
    String,
>;

/// The settings read again: the theme and the fonts drawn with, the switches of the timeline in force, the level of the
/// Command log; a change to a setting that changes what is read reads the repository again.
fn settings_read(state: &mut State, settings: Settings) -> Vec<Effect> {
    let looks_different = settings.theme != state.settings.theme || settings.fonts != state.settings.fonts;
    let before = state.settings.switches;
    let sorted_before = state.settings.sort_order;
    let changes_reads = settings_page::changes_reads(&state.settings, &settings);
    state.settings = settings;
    new_worktrees::settings_changed(state);
    state.log.set_level(state.settings.log_level);
    if state.settings.sort_order != sorted_before {
        rail::arrange(state);
    }
    let mut effects = tabs::settings_changed(state);
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

/// A folder chosen in the system's dialog, which the platform shows for [`Effect::ChooseFolder`]: shown at once in a
/// window with no workspace; otherwise **Replace the workspace** or **Add to the workspace** is asked, and answered by
/// `answer_open_folder`.
///
/// Feature: repo.open-folder
fn open_folder(state: &mut State, folder: Option<PathBuf>) -> Vec<Effect> {
    let Some(folder) = folder else {
        return Vec::new();
    };
    open_folders(
        state,
        Opened {
            folders: vec![folder],
            file: None,
        },
    )
}

/// Folders chosen, from a folder or from a workspace file of VS Code: shown at once in a window with no workspace;
/// otherwise **Replace the workspace** or **Add to the workspace** is asked.
fn open_folders(state: &mut State, opened: Opened) -> Vec<Effect> {
    if state.workspace.is_some() {
        state.dialog = Some(Dialog::OpenFolder { opened });
        return Vec::new();
    }
    let request = OpenRequest {
        is_shown_at_once: true,
        file: opened.file,
        ..OpenRequest::default()
    };
    workspace::open_workspace(state, opened.folders, request)
}

/// The answer to **Replace the workspace** or **Add to the workspace**: the window keeps what it shows until the
/// repositories are found; an added folder keeps the repository shown, and the tabs open.
fn answer_open_folder(state: &mut State, answer: FolderAnswer) -> Vec<Effect> {
    let Some(Dialog::OpenFolder { opened }) = state.dialog.take() else {
        return Vec::new();
    };
    match answer {
        FolderAnswer::Cancel => Vec::new(),
        FolderAnswer::Replace => {
            let request = OpenRequest {
                file: opened.file,
                ..OpenRequest::default()
            };
            workspace::open_workspace(state, opened.folders, request)
        }
        FolderAnswer::Add => {
            let mut folders = state
                .workspace
                .as_ref()
                .map_or_default(|workspace| workspace.folders.clone());
            for folder in opened.folders {
                if !folders.iter().any(|known| persisted::same_folder(known, &folder)) {
                    folders.push(folder);
                }
            }
            let request = OpenRequest {
                remembered: state.shown_folder().cloned(),
                is_added: true,
                ..OpenRequest::default()
            };
            workspace::open_workspace(state, folders, request)
        }
    }
}

/// What a view asked of the workspace: a folder, a workspace file or a recent workspace to open, a repository to show.
fn reduce_workspace_intent(state: &mut State, intent: Intent) -> Vec<Effect> {
    match intent {
        Intent::OpenFolder => {
            state.dialog = None;
            vec![Effect::ChooseFolder]
        }
        Intent::OpenWorkspaceFile => {
            state.dialog = None;
            vec![Effect::ChooseWorkspaceFile]
        }
        Intent::AnswerOpenFolder(answer) => answer_open_folder(state, answer),
        Intent::OpenRecent(index) => open_recent(state, index),
        Intent::SelectRepository(folder) => tabs::show_repository(state, &folder),
        Intent::PickRepository => {
            workspace::pick_repository(state);
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// What a view returned.
fn reduce_intent(state: &mut State, intent: Intent) -> Vec<Effect> {
    match intent {
        Intent::OpenFolder
        | Intent::AnswerOpenFolder(_)
        | Intent::OpenRecent(_)
        | Intent::SelectRepository(_)
        | Intent::OpenWorkspaceFile
        | Intent::PickRepository => reduce_workspace_intent(state, intent),
        Intent::SwitchTab(_)
        | Intent::ShowInTab(..)
        | Intent::FoldTab(_)
        | Intent::OpenTab(_)
        | Intent::PickTabToOpen
        | Intent::TabsChosen(_)
        | Intent::CloseTab(_)
        | Intent::CloseOtherTabs(_)
        | Intent::CloseTabShown
        | Intent::RevealTab(_) => tasks_tab::close_tabs(state, intent),
        Intent::ReadRepository | Intent::CheckAgain | Intent::Refresh => read_again(state, &intent),
        Intent::PickTheme
        | Intent::CloseThemePicker
        | Intent::SetSetting(_)
        | Intent::ChooseSettingsFile(_)
        | Intent::Settings(_) => settings_page::reduce_intent(state, &intent),
        Intent::ShowTimeline | Intent::ShowPage(_) | Intent::NextPage | Intent::BackTo(_) => {
            pages::reduce(state, &intent)
        }
        Intent::ClosePage => file_page::close_page(state),
        Intent::ShowCommandLog | Intent::Panel(_) => panel::reduce_intent(state, intent),
        Intent::Terminal(intent) => terminals::reduce(state, intent),
        Intent::ClosePicker => {
            state.dialog = None;
            Vec::new()
        }
        Intent::OpenUrl(url) => vec![Effect::OpenUrl(url)],
        Intent::DismissNotice => dismiss_notice(state),
        Intent::Rail(intent) => pages::starting_again(state, |state| rail::reduce_rail(state, intent)),
        Intent::Filter(intent) => rail::reduce_filter(state, intent),
        Intent::Graph(intent) => reduce_graph(state, intent),
        Intent::Search(intent) => search::reduce(state, intent),
        Intent::Details(intent) => inspector::reduce_details(state, intent),
        Intent::CommitPage(id, intent) => commit_page::reduce_page(state, id, intent),
        Intent::Diff(intent) => diff_page::reduce(state, intent),
        Intent::MergeView(intent) => merge_page::reduce(state, intent),
        Intent::FileView(_) | Intent::Files(_) | Intent::Blame(_) | Intent::FileSearch(_) => {
            files_page::reduce_file_intent(state, intent)
        }
        Intent::Compare(intent) => compare::reduce(state, intent),
        Intent::Review(intent) => review::reduce(state, intent),
        Intent::Feature(intent) => feature::reduce(state, intent),
        Intent::Changes(intent) => changes::reduce(state, intent),
        Intent::Rebase(intent) => rebase::reduce(state, intent),
        Intent::RangeDiff(intent) => range_diff::reduce(state, intent),
        Intent::Conflicts(intent) => conflicts::reduce(state, intent),
        Intent::WorktreeSetup(intent) => new_worktrees::reduce(state, intent),
        Intent::TasksTab(intent) => tasks_tab::reduce(state, intent),
        Intent::Window(intent) => vec![Effect::Window(intent)],
    }
}

/// The repository read again: once when it was not read on startup, by **Check again**, or by **Refresh**, which asks
/// for a read after the one under way.
fn read_again(state: &mut State, intent: &Intent) -> Vec<Effect> {
    if *intent == Intent::ReadRepository || state.reads.is_not_read {
        state.reads.is_not_read = false;
        return look_again(state, false);
    }
    if *intent == Intent::Refresh && state.reads.schedule.is_reading() {
        return reads::asked(state, model::repo::ReadCause::Refresh);
    }
    look_again(state, *intent == Intent::Refresh)
}

/// Puts the notice shown away.
fn dismiss_notice(state: &mut State) -> Vec<Effect> {
    state.notice = None;
    Vec::new()
}

/// An intent of the timeline; selecting a row there closes the comparison shown in place in the inspector.
fn reduce_graph(state: &mut State, intent: views::graph::GraphIntent) -> Vec<Effect> {
    if matches!(intent, views::graph::GraphIntent::Select(_)) {
        state.compares.in_place = None;
    }
    timeline::reduce_graph(state, intent)
}

/// Opens a recent workspace, with the repository shown in it last; the window keeps what it shows until its
/// repositories are found, since one whose folders are all gone is refused.
///
/// Feature: repo.open-recent
fn open_recent(state: &mut State, index: usize) -> Vec<Effect> {
    let Some(recent) = state.persisted.recent.get(index).cloned() else {
        return Vec::new();
    };
    let request = OpenRequest {
        remembered: recent.repository,
        recent: Some(index),
        file: recent.file,
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
        is_shown_at_once: !is_refresh,
        is_refresh,
        ..OpenRequest::default()
    };
    workspace::open_workspace(state, folders, request)
}
