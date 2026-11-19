//! The jobs of the workers on real repositories: the repositories of a workspace found, the one shown read, a folder
//! gone, and the settings files watched.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use app::settings::{SettingsFiles, repository_settings_file};
use app::store::{Found, Message, Shown, ViewData};
use app::work::jobs::find_repositories;
use app::work::{Deliver, SettingsWatcher};
use git::CancelToken;
use render::ThemeName;
use views::Intent;
use views::graph::HeadName;

use super::support::run;
use super::{repository_with_history, showing};

#[test]
fn repo_read_state__the_header_shows_the_branch_checked_out_and_how_far_it_is_from_its_upstream() {
    let (repository, _remote) = repository_with_history();
    let state = showing(repository.path().to_path_buf());
    let read = state.shown_read().unwrap();
    assert_eq!(read.state.branches.len(), 2);
    assert_eq!(state.timeline.loaded(), 4, "three commits and the stash");
    let data = ViewData::new(&state, Vec::new(), super::support::NOW);
    let header = data.window_state(&state).header;
    let place = header.place.unwrap();
    assert_eq!(place.head, Some(HeadName::Branch("main")));
    assert_eq!(header.sync.map(|sync| (sync.ahead, sync.behind)), Some((1, 0)));
}

#[test]
fn repo_no_repository__running_git_init_and_check_again_shows_the_new_repository_with_no_commits() {
    let holder = testkit::Repository::new();
    let folder = holder.path().parent().unwrap().join("later");
    std::fs::create_dir_all(&folder).unwrap();
    let mut state = showing(folder.clone());
    assert_eq!(state.workspace.as_ref().unwrap().shown, Shown::NoRepository);
    let made = std::process::Command::new("git")
        .args(["init", "--quiet", "-b", "main"])
        .current_dir(&folder)
        .status()
        .unwrap();
    assert!(made.success());
    let _effects = run(&mut state, Message::Intent(Intent::CheckAgain));
    let read = state.shown_read().expect("the new repository is shown");
    assert!(read.state.head.commit.is_none());
    assert_eq!(state.timeline.rows, [] as [model::graph::Row; 0]);
    let data = ViewData::new(&state, Vec::new(), super::support::NOW);
    assert_eq!(
        data.window_state(&state).header.place.and_then(|place| place.head),
        Some(HeadName::Unborn("main")),
        "the header says main (no commits yet)"
    );
}

#[test]
fn repo_no_repository__deleting_the_folder_of_the_repository_shown_brings_the_window_to_it() {
    let (repository, remote) = repository_with_history();
    let folder = repository.path().to_path_buf();
    let mut state = showing(folder.clone());
    assert!(state.shown_read().is_some());
    drop(remote);
    std::fs::remove_dir_all(&folder).unwrap();
    let _effects = run(&mut state, Message::Intent(Intent::Refresh));
    assert_eq!(state.workspace.as_ref().unwrap().shown, Shown::NoRepository);
}

#[test]
fn repo_open_recent__the_folders_of_a_workspace_all_gone_are_found_gone() {
    let holder = testkit::Repository::new();
    let gone = holder.path().parent().unwrap().join("gone");
    assert_eq!(find_repositories(&[gone], &CancelToken::new()), Found::AllGone);
    let found = find_repositories(&[holder.path().to_path_buf()], &CancelToken::new());
    assert!(matches!(found, Found::Repositories(repositories) if repositories.len() == 1));
}

#[test]
fn settings_read__a_change_to_either_file_is_applied_within_a_second() {
    let repository = testkit::Repository::new();
    let user = repository.path().parent().unwrap().join("user-settings.json");
    std::fs::write(&user, r#"{ "theme": "dark" }"#).unwrap();
    let (sender, receiver) = mpsc::channel();
    let sender = Arc::new(Mutex::new(sender));
    let deliver: Deliver = Arc::new(move |message| {
        let _sent = sender.lock().unwrap().send(message);
    });
    let watcher = SettingsWatcher::new(deliver);
    watcher.watch(SettingsFiles {
        user: Some(user.clone()),
        repository: Some(repository_settings_file(repository.path())),
    });
    let theme = |message: Message| match message {
        Message::SettingsRead(settings) => settings.theme,
        other => panic!("not settings: {other:?}"),
    };
    assert_eq!(
        theme(receiver.recv_timeout(Duration::from_secs(1)).unwrap()),
        ThemeName::Dark
    );
    let changed = Instant::now();
    std::fs::write(&user, r#"{ "theme": "light", "font.size": 14 }"#).unwrap();
    assert_eq!(
        theme(receiver.recv_timeout(Duration::from_secs(1)).unwrap()),
        ThemeName::Light
    );
    assert!(changed.elapsed() < Duration::from_secs(1));
    let shared = repository_settings_file(repository.path());
    std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
    std::fs::write(&shared, r#"{ "graph.pageSize": 200 }"#).unwrap();
    let Message::SettingsRead(settings) = receiver.recv_timeout(Duration::from_secs(1)).unwrap() else {
        panic!("the settings are read again");
    };
    assert_eq!(settings.page_size, model::graph::PageSize::from_setting(Some(200.0)));
}
