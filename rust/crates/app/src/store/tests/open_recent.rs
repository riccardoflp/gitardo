//! `repo.open-recent`: a workspace opened before opens again with the repository shown in it last.

use views::{Intent, Notice};

use super::{act, answer, folder, intent, open, reads, timeline_folder, window};
use crate::persisted::RecentWorkspace;
use crate::store::{Effect, Found, JobKind, Message, reduce};

#[test]
fn repo_open_recent__a_workspace_opened_before_shows_its_tabs_and_the_repository_shown_in_it_last() {
    let mut state = window();
    let (work, other) = (folder("work"), folder("other"));
    let found = [
        (work.as_path(), &[work.join("app"), work.join("lib")][..]),
        (other.as_path(), std::slice::from_ref(&other)),
    ];
    let _opened = open(&mut state, &work, &found);
    let _chosen = act(&mut state, Intent::SelectRepository(work.join("lib")), &found);
    let _asked = reduce(&mut state, Message::FolderChosen(Some(other.clone())));
    let _replaced = act(
        &mut state,
        Intent::AnswerOpenFolder(views::FolderAnswer::Replace),
        &found,
    );
    assert_eq!(state.persisted.recent[1].folders, std::slice::from_ref(&work));
    let effects = intent(&mut state, Intent::OpenRecent(1));
    assert_eq!(
        timeline_folder(&state),
        Some(other.clone()),
        "the window keeps what it shows until the folders are found"
    );
    let effects = answer(&mut state, effects, &found);
    assert_eq!(reads(&effects).first(), Some(&work.join("lib")));
    assert_eq!(timeline_folder(&state), Some(work.join("lib")));
    let tabs: Vec<_> = state.tabs.open.iter().map(|tab| tab.folder.clone()).collect();
    assert_eq!(tabs, [work.join("app"), work.join("lib")], "with the tabs it had");
    assert_eq!(state.persisted.recent[0].folders, [work]);
}

#[test]
fn repo_open_recent__a_workspace_whose_folders_are_gone_is_refused_and_leaves_the_list() {
    let mut state = window();
    state.persisted.recent = vec![
        RecentWorkspace {
            folders: vec![folder("gone")],
            repository: None,
            ..RecentWorkspace::default()
        },
        RecentWorkspace {
            folders: vec![folder("work")],
            repository: None,
            ..RecentWorkspace::default()
        },
    ];
    let effects = intent(&mut state, Intent::OpenRecent(0));
    let Some(Effect::Start(job)) = effects.last() else {
        panic!("the folders are looked for");
    };
    assert!(matches!(&job.kind, JobKind::FindRepositories { folders } if *folders == [folder("gone")]));
    let effects = reduce(
        &mut state,
        Message::RepositoriesFound {
            job: job.id,
            found: Found::AllGone,
        },
    );
    assert_eq!(state.notice, Some(Notice::RecentGone));
    assert_eq!(state.persisted.recent.len(), 1);
    assert_eq!(state.persisted.recent[0].folders, [folder("work")]);
    assert_eq!(effects, [Effect::WritePersisted(state.persisted.clone())]);
    assert!(state.workspace.is_none());
    let _dismissed = intent(&mut state, Intent::DismissNotice);
    assert_eq!(state.notice, None);
}

#[test]
fn repo_open_recent__a_workspace_opened_from_the_command_line_shows_its_remembered_repository() {
    let mut state = window();
    let work = folder("work");
    state.persisted.recent = vec![RecentWorkspace {
        folders: vec![work.clone()],
        repository: Some(work.join("lib")),
        ..RecentWorkspace::default()
    }];
    let effects = reduce(
        &mut state,
        Message::Started {
            folders: vec![work.clone()],
        },
    );
    let effects = answer(&mut state, effects, &[(&work, &[work.join("app"), work.join("lib")])]);
    assert_eq!(reads(&effects).first(), Some(&work.join("lib")));
}
