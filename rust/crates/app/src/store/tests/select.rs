//! `repo.select`: the repository shown, among those of the workspace.

use views::{Intent, Notice};

use super::{answer, folder, intent, open, reads, started, tabs_listed, timeline_folder, window};
use crate::persisted::RecentWorkspace;
use crate::store::{Dialog, Effect, Message, Shown, reduce};

#[test]
fn repo_select__choosing_the_second_shows_it_and_it_is_shown_the_next_time() {
    let mut state = window();
    let work = folder("work");
    let found = [(work.as_path(), &[work.join("app"), work.join("lib")][..])];
    let _opened = open(&mut state, &work, &found);
    let effects = intent(&mut state, Intent::SelectRepository(work.join("lib")));
    assert!(
        reads(&effects).is_empty(),
        "lib was read once the tab shown was (`repo.watch-hidden`)"
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::WatchSettings(files)
        if files.repository == Some(work.join("lib").join(".gitardo").join("settings.json"))))
    );
    let _read = answer(&mut state, effects, &found);
    assert_eq!(timeline_folder(&state), Some(work.join("lib")));
    let mut next_time = window();
    next_time.persisted = state.persisted.clone();
    let effects = reduce(
        &mut next_time,
        Message::Started {
            folders: vec![work.clone()],
        },
    );
    let effects = answer(&mut next_time, effects, &found);
    assert_eq!(reads(&effects).first(), Some(&work.join("lib")));
    assert_eq!(timeline_folder(&next_time), Some(work.join("lib")));
}

#[test]
fn repo_select__a_remembered_repository_no_longer_in_the_workspace_gives_way_to_the_first() {
    let mut state = window();
    let work = folder("work");
    state.persisted.recent = vec![RecentWorkspace {
        folders: vec![work.clone()],
        repository: Some(work.join("deleted")),
        ..RecentWorkspace::default()
    }];
    let effects = reduce(
        &mut state,
        Message::Started {
            folders: vec![work.clone()],
        },
    );
    let effects = answer(&mut state, effects, &[(&work, &[work.join("app"), work.join("lib")])]);
    assert_eq!(reads(&effects).first(), Some(&work.join("app")));
    assert_eq!(state.persisted.recent[0].repository, Some(work.join("app")));
}

#[test]
fn repo_select__with_one_repository_the_island_has_one_tab_and_the_command_says_so() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    assert_eq!(tabs_listed(&state), [(String::from("work"), true)]);
    assert_eq!(intent(&mut state, Intent::PickRepository).len(), 0);
    assert_eq!(state.notice, Some(Notice::SingleRepository));
    assert_eq!(state.dialog, None);
    let mut empty = window();
    let _opened = open(&mut empty, &folder("empty"), &[]);
    let _asked = intent(&mut empty, Intent::PickRepository);
    assert_eq!(empty.notice, Some(Notice::NoRepositoryToSelect));
}

#[test]
fn repo_select__the_picker_opens_with_several_repositories() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, &[work.join("app"), work.join("lib")])]);
    let _asked = intent(&mut state, Intent::PickRepository);
    assert_eq!(state.dialog, Some(Dialog::Picker));
    let effects = intent(&mut state, Intent::SelectRepository(work.join("lib")));
    assert_eq!(state.dialog, None);
    assert!(
        reads(&effects).is_empty(),
        "lib was read once the tab shown was (`repo.watch-hidden`)"
    );
    assert_eq!(timeline_folder(&state), Some(work.join("lib")));
}

#[test]
fn repo_select__choosing_the_repository_shown_reads_nothing() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, &[work.join("app"), work.join("lib")])]);
    let before = state.clone();
    assert_eq!(intent(&mut state, Intent::SelectRepository(work.join("app"))).len(), 0);
    assert_eq!(state, before);
}

#[test]
fn repo_select__while_an_operation_runs_choosing_another_does_nothing() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, &[work.join("app"), work.join("lib")])]);
    state.is_operation_running = true;
    let before = state.clone();
    // In Gitardo Desktop the tabs are shown while an operation runs (`repo.switch-tab`); the choice itself waits.
    assert_eq!(crate::store::workspace::select(&mut state, &work.join("lib")).len(), 0);
    assert_eq!(state, before);
}

#[test]
fn repo_select__the_read_of_a_repository_no_longer_chosen_is_cancelled_and_its_result_dropped() {
    let mut state = window();
    // With `repositories.keepOpen` on, the repository chosen again shows what it kept (`repo.keep-open`).
    state.settings.keep_open = false;
    let work = folder("work");
    let found = [(work.as_path(), &[work.join("app"), work.join("lib")][..])];
    let _opened = open(&mut state, &work, &found);
    let first = intent(&mut state, Intent::SelectRepository(work.join("lib")));
    let second = intent(&mut state, Intent::SelectRepository(work.join("app")));
    let (late, _) = started(&first)[0].clone();
    assert!(second.contains(&Effect::Cancel(late)));
    let dropped = reduce(
        &mut state,
        Message::RepositoryRead {
            job: late,
            folder: work.join("lib"),
            result: Err(String::from("cancelled")),
            read_at: 0,
        },
    );
    assert_eq!(dropped.len(), 0);
    assert_eq!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(&Shown::Reading {
            folder: work.join("app")
        })
    );
}
