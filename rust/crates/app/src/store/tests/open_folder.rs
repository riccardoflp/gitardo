//! `repo.open-folder`: a folder chosen replaces the workspace or is added to it.

use views::repo::NoRepositoryState;
use views::{Content, FolderAnswer, Intent, TimelineState};

use super::{act, answer, folder, intent, open, reads, tabs_listed, timeline_folder, view_data, window};
use crate::store::{Dialog, Effect, Message, reduce};

#[test]
fn repo_open_folder__a_folder_with_two_repositories_shows_the_first_and_has_a_tab_for_each() {
    let mut state = window();
    let work = folder("work");
    let repositories = [work.join("app"), work.join("lib")];
    let effects = open(&mut state, &work, &[(&work, &repositories)]);
    assert_eq!(
        reads(&effects),
        [work.join("app"), work.join("lib")],
        "the tab shown first, then the other in the background"
    );
    assert_eq!(timeline_folder(&state), Some(work.join("app")));
    assert_eq!(
        tabs_listed(&state),
        [(String::from("work/app"), true), (String::from("work/lib"), false)]
    );
    assert_eq!(view_data(&state).name(), Some("work/app"));
}

#[test]
fn repo_open_folder__adding_a_folder_keeps_the_repository_shown() {
    let mut state = window();
    let (work, other) = (folder("work"), folder("other"));
    let found = [
        (work.as_path(), &[work.join("lib"), work.join("app")][..]),
        (other.as_path(), std::slice::from_ref(&other)),
    ];
    let _opened = open(&mut state, &work, &found);
    let _chosen = act(&mut state, Intent::SelectRepository(work.join("app")), &found);
    let generation = state.shown_read().map(|read| read.generation);
    assert_eq!(reduce(&mut state, Message::FolderChosen(Some(other.clone()))).len(), 0);
    assert_eq!(
        state.dialog,
        Some(Dialog::OpenFolder {
            opened: crate::store::Opened {
                folders: vec![other.clone()],
                file: None,
            }
        })
    );
    let effects = act(&mut state, Intent::AnswerOpenFolder(FolderAnswer::Add), &found);
    assert!(
        !reads(&effects).contains(&work.join("app")),
        "the repository shown is not read again"
    );
    assert_eq!(timeline_folder(&state), Some(work.join("app")));
    assert_eq!(state.shown_read().map(|read| read.generation), generation);
    let workspace = state.workspace.as_ref().unwrap();
    assert_eq!(workspace.folders, [work.clone(), other.clone()]);
    assert_eq!(state.repositories().len(), 3);
}

#[test]
fn repo_open_folder__replacing_the_workspace_shows_the_folder_opened() {
    let mut state = window();
    let (work, other) = (folder("work"), folder("other"));
    let found = [
        (work.as_path(), std::slice::from_ref(&work)),
        (other.as_path(), std::slice::from_ref(&other)),
    ];
    let _opened = open(&mut state, &work, &found);
    let _asked = reduce(&mut state, Message::FolderChosen(Some(other.clone())));
    let effects = intent(&mut state, Intent::AnswerOpenFolder(FolderAnswer::Replace));
    assert_eq!(
        timeline_folder(&state),
        Some(work.clone()),
        "the window shows what it showed until the repositories are found"
    );
    let _read = answer(&mut state, effects, &found);
    assert_eq!(state.workspace.as_ref().unwrap().folders, std::slice::from_ref(&other));
    assert_eq!(timeline_folder(&state), Some(other));
}

#[test]
fn repo_open_folder__cancelling_the_question_leaves_the_workspace() {
    let mut state = window();
    let (work, other) = (folder("work"), folder("other"));
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let before = state.clone();
    let _asked = reduce(&mut state, Message::FolderChosen(Some(other)));
    assert_eq!(
        intent(&mut state, Intent::AnswerOpenFolder(FolderAnswer::Cancel)).len(),
        0
    );
    assert_eq!(state, before);
    assert_eq!(reduce(&mut state, Message::FolderChosen(None)).len(), 0);
    assert_eq!(intent(&mut state, Intent::OpenFolder), [Effect::ChooseFolder]);
}

#[test]
fn repo_open_folder__a_folder_with_no_repository_shows_no_repository() {
    let mut state = window();
    let empty = folder("empty");
    let effects = open(&mut state, &empty, &[]);
    assert_eq!(reads(&effects).len(), 0);
    let data = view_data(&state);
    assert_eq!(
        data.window_state(&state).content,
        Content::Timeline(TimelineState::NoRepository(NoRepositoryState::NoRepository))
    );
    assert_eq!(view_data(&state).name(), Some("empty"));
    assert_eq!(tabs_listed(&state), []);
}

#[test]
fn repo_open_folder__the_workspace_opened_is_the_first_recent_one_on_the_welcome_page() {
    let mut state = window();
    let (work, other) = (folder("work"), folder("other"));
    let _first = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let _asked = reduce(&mut state, Message::FolderChosen(Some(other.clone())));
    let effects = intent(&mut state, Intent::AnswerOpenFolder(FolderAnswer::Replace));
    let effects = answer(&mut state, effects, &[(&other, std::slice::from_ref(&other))]);
    assert!(effects.iter().any(|effect| matches!(effect, Effect::WritePersisted(_))));
    let _welcome = intent(&mut state, Intent::ShowPage(views::page::Page::Welcome));
    let data = view_data(&state);
    let Content::Page(page) = data.window_state(&state).content else {
        panic!("the Welcome page is shown");
    };
    let views::page::PageState::Welcome(welcome) = page.page else {
        panic!("the Welcome page is not shown");
    };
    assert_eq!(
        welcome.recent,
        [other.display().to_string(), work.display().to_string()]
    );
}
