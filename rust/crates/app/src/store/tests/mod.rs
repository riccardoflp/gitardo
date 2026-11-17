//! The tests of the store: `reduce` with the answers of the workers made by hand, and what the views are given.

#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, clippy::panic, reason = "a test that cannot go on fails")]

mod code_workspace;
mod github;
mod hidden;
mod open_folder;
mod open_recent;
mod pages;
mod reads;
mod select;
mod tabs;

use std::path::{Path, PathBuf};

use git::read::graph::CommitPage;
use model::CommitId;
use model::repo::Repository;
use model::state::{Branch, Head, RepositoryState};
use views::{Content, Intent, TimelineState};

use super::{Effect, Found, JobId, JobKind, Message, State, ViewData, reduce};
use crate::persisted::Persisted;
use crate::settings::Settings;

/// A window that started before, with no workspace.
fn window() -> State {
    State::new(
        Persisted {
            has_started: true,
            ..Persisted::default()
        },
        Settings::default(),
        None,
    )
}

/// A folder for the tests; nothing is read from it.
fn folder(name: &str) -> PathBuf {
    PathBuf::from(format!("/work/{name}"))
}

/// The jobs an answer started.
fn started(effects: &[Effect]) -> Vec<(JobId, JobKind)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Start(job) => Some((job.id, job.kind.clone())),
            _ => None,
        })
        .collect()
}

/// The folders whose repository an answer started to read.
fn reads(effects: &[Effect]) -> Vec<PathBuf> {
    started(effects)
        .into_iter()
        .filter_map(|(_, kind)| match kind {
            JobKind::ReadRepository { folder, .. } => Some(folder),
            JobKind::FindRepositories { .. }
            | JobKind::ReadCommits { .. }
            | JobKind::ReadTimeline { .. }
            | JobKind::Search(_)
            | JobKind::ReadDetails { .. }
            | JobKind::ReadCommitPage { .. }
            | JobKind::ReadDiff { .. }
            | JobKind::ReadMerge { .. }
            | JobKind::ReadFile { .. }
            | JobKind::ReadComparison { .. }
            | JobKind::ReadInPlace { .. }
            | JobKind::ReadReview { .. }
            | JobKind::ReadFiles { .. }
            | JobKind::ReadNewWorktrees { .. }
            | JobKind::ReadIgnored { .. }
            | JobKind::ReadBlame { .. }
            | JobKind::CountConflicts { .. } => None,
        })
        .collect()
}

/// A repository with one branch `main`, checked out at a commit.
fn repository_state(folder: &Path) -> RepositoryState {
    let commit = CommitId::parse("4b830a6c0ffee000000000000000000000000000").unwrap();
    RepositoryState {
        folder: folder.to_path_buf(),
        head: Head {
            commit: Some(commit.clone()),
            branch: Some(String::from("refs/heads/main")),
        },
        branches: vec![Branch {
            name: String::from("main"),
            commit,
            upstream: None,
            is_checked_out: true,
            date: model::Timestamp(0),
        }],
        ..RepositoryState::default()
    }
}

/// Answers the jobs an answer started as a worker would: the repositories of each folder as given, each repository
/// read with one branch, every repository offered in tabs opened (`repo.choose-tabs`). Goes on with the jobs those
/// answers start, and gives every effect.
fn answer(state: &mut State, effects: Vec<Effect>, found: &[(&Path, &[PathBuf])]) -> Vec<Effect> {
    let mut all = answer_asking(state, effects, found);
    if let Some(super::Dialog::ChooseTabs(choosing)) = &state.dialog {
        let every = (0..choosing.offered.len()).collect();
        let effects = intent(state, Intent::TabsChosen(Some(every)));
        all.extend(answer_asking(state, effects, found));
    }
    all
}

/// Answers the jobs an answer started as [`answer`] does, leaving the picker of the tabs to open open.
fn answer_asking(state: &mut State, effects: Vec<Effect>, found: &[(&Path, &[PathBuf])]) -> Vec<Effect> {
    let mut all = Vec::new();
    let mut waiting = effects;
    while !waiting.is_empty() {
        let jobs = started(&waiting);
        all.append(&mut waiting);
        for (job, kind) in jobs {
            let message = match kind {
                JobKind::FindRepositories { folders } => Message::RepositoriesFound {
                    job,
                    found: Found::Repositories(
                        folders
                            .iter()
                            .flat_map(|asked| {
                                found
                                    .iter()
                                    .filter(move |(folder, _)| folder == asked)
                                    .flat_map(|(_, repositories)| repositories.iter())
                            })
                            .map(|folder| Repository {
                                folder: folder.clone(),
                                superproject: None,
                                common_folder: None,
                            })
                            .collect(),
                    ),
                },
                JobKind::ReadRepository { folder, .. } => Message::RepositoryRead {
                    job,
                    result: Ok(Box::new((repository_state(&folder), CommitPage::default()))),
                    folder,
                    read_at: 0,
                },
                other => not_read(job, &other),
            };
            waiting.extend(reduce(state, message));
        }
    }
    all
}

/// What a list of the files of the Files page, a read of the page of new worktrees, or a question of the files git
/// ignores gives in these tests: nothing.
fn not_listed(job: JobId, kind: &JobKind) -> Message {
    if let JobKind::ReadNewWorktrees { .. } = kind {
        return Message::NewWorktreesRead {
            job,
            result: Box::new(crate::work::new_worktrees::SetupReadout {
                in_effect: model::worktree_setup::settings_in_effect(&model::worktree_setup::SettingsInput::default()),
                repository: crate::work::new_worktrees::PlaceLists::default(),
                user: crate::work::new_worktrees::PlaceLists::default(),
                look: None,
            }),
        };
    }
    if let JobKind::ReadIgnored { .. } = kind {
        return Message::IgnoredRead {
            job,
            result: Err(String::from("not read in these tests")),
        };
    }
    Message::FilesRead {
        job,
        result: Err(String::from("not read in these tests")),
    }
}

/// The answer to a job these tests do not read: a failure, or an empty page of the timeline.
fn not_read(job: JobId, kind: &JobKind) -> Message {
    match kind {
        JobKind::ReadCommits { .. } | JobKind::FindRepositories { .. } | JobKind::ReadRepository { .. } => {
            Message::CommitsRead {
                job,
                result: Err(String::from("not read in these tests")),
            }
        }
        JobKind::ReadTimeline { .. } => Message::TimelineRead {
            job,
            result: Ok(CommitPage::default()),
        },
        JobKind::Search(_) => Message::Searched {
            job,
            result: crate::store::search::SearchResult::Matches(Err(String::from("not read in these tests"))),
        },
        JobKind::ReadDetails { .. } => Message::DetailsRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadCommitPage { .. } => Message::CommitPageRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadDiff { .. } => Message::DiffRead {
            job,
            result: crate::work::diff::DiffRead::Failed(String::from("not read in these tests")),
        },
        JobKind::ReadMerge { .. } => Message::MergeRead {
            job,
            result: crate::work::merge_view::MergeRead::Failed(String::from("not read in these tests")),
        },
        JobKind::ReadReview { .. } => Message::ReviewRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadFile { .. } => Message::FileRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadComparison { .. } => Message::ComparisonRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadInPlace { .. } => Message::InPlaceRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::CountConflicts { .. } => Message::ConflictsCounted {
            job,
            result: Err(String::from("not read in these tests")),
        },
        JobKind::ReadFiles { .. } | JobKind::ReadNewWorktrees { .. } | JobKind::ReadIgnored { .. } => {
            not_listed(job, kind)
        }
        JobKind::ReadBlame { .. } => Message::BlameRead {
            job,
            result: Err(String::from("not read in these tests")),
        },
    }
}

/// Opens a folder in a window with no workspace, and answers its jobs.
fn open(state: &mut State, opened: &Path, found: &[(&Path, &[PathBuf])]) -> Vec<Effect> {
    let effects = reduce(state, Message::FolderChosen(Some(opened.to_path_buf())));
    answer(state, effects, found)
}

/// What the views are given, with nothing running.
fn view_data(state: &State) -> ViewData {
    ViewData::new(state, Vec::new(), 0)
}

/// The tabs the island of the tabs is given: the name of each, and whether it is the one shown.
fn tabs_listed(state: &State) -> Vec<(String, bool)> {
    let data = view_data(state);
    let window = data.window_state(state);
    window
        .side
        .map_or_default(|side| side.tabs.tabs)
        .iter()
        .map(|tab| (tab.name.clone(), tab.is_shown))
        .collect()
}

/// The folder of the repository the timeline shows, once read.
fn timeline_folder(state: &State) -> Option<PathBuf> {
    let data = view_data(state);
    let window = data.window_state(state);
    match (window.content, window.side.and_then(|side| side.branches)) {
        (Content::Timeline(TimelineState::Repository { .. }), Some(rail)) => Some(rail.repository.folder.clone()),
        _ => None,
    }
}

/// Reduces an intent.
fn intent(state: &mut State, intent: Intent) -> Vec<Effect> {
    reduce(state, Message::Intent(intent))
}

/// Reduces an intent and answers the jobs it starts.
fn act(state: &mut State, wanted: Intent, found: &[(&Path, &[PathBuf])]) -> Vec<Effect> {
    let effects = intent(state, wanted);
    answer(state, effects, found)
}

/// Reduces a message and answers the jobs it starts.
fn send(state: &mut State, message: Message, found: &[(&Path, &[PathBuf])]) -> Vec<Effect> {
    let effects = reduce(state, message);
    answer(state, effects, found)
}
