//! `welcome.page` and the pages of the window, `repo.no-repository`, and the messages that are not about the
//! workspace: the settings read again, the window closed.

use render::ThemeName;
use views::graph::GraphIntent;
use views::page::{Page, PageState};
use views::repo::NoRepositoryState;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::{Content, Intent, RailIntent, RailList, TimelineState};

use super::{answer, folder, intent, open, reads, send, view_data, window};
use crate::persisted::{Persisted, RecentWorkspace};
use crate::settings::Settings;
use crate::store::{Effect, Message, Shown, State, reduce};

/// The page shown, and whether it has its trail; none when the timeline is shown.
fn page_shown(state: &State) -> Option<(Page, bool)> {
    let data = view_data(state);
    match data.window_state(state).content {
        Content::Page(host) => Some((host.page.page(), host.has_trail)),
        Content::Timeline(_) => None,
    }
}

#[test]
fn welcome_page__the_first_start_shows_it_and_the_second_does_not() {
    let mut first = State::new(Persisted::default(), Settings::default(), None);
    let effects = reduce(&mut first, Message::Started { folders: Vec::new() });
    assert_eq!(page_shown(&first), Some((Page::Welcome, false)));
    assert!(effects.contains(&Effect::WritePersisted(first.persisted.clone())));
    let work = folder("work");
    let _opened = send(
        &mut first,
        Message::FolderChosen(Some(work.clone())),
        &[(&work, std::slice::from_ref(&work))],
    );
    assert_eq!(
        page_shown(&first),
        None,
        "the workspace opened from the page shows its timeline"
    );
    let mut second = State::new(first.persisted.clone(), Settings::default(), None);
    let effects = reduce(
        &mut second,
        Message::Started {
            folders: vec![work.clone()],
        },
    );
    let _read = answer(&mut second, effects, &[(&work, std::slice::from_ref(&work))]);
    assert_eq!(page_shown(&second), None);
}

#[test]
fn welcome_page__the_first_start_on_a_folder_shows_it_over_the_workspace() {
    let mut first = State::new(Persisted::default(), Settings::default(), None);
    let work = folder("work");
    let effects = reduce(
        &mut first,
        Message::Started {
            folders: vec![work.clone()],
        },
    );
    assert_eq!(page_shown(&first), Some((Page::Welcome, true)));
    let _read = answer(&mut first, effects, &[(&work, std::slice::from_ref(&work))]);
    let _back = intent(&mut first, Intent::ShowTimeline);
    assert_eq!(page_shown(&first), None);
}

#[test]
fn welcome_page__shown_again_it_has_the_steps_done_ticked() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    assert!(state.persisted.welcome_steps[0], "the timeline was shown");
    let _shown = intent(&mut state, Intent::ShowPage(Page::Welcome));
    let data = view_data(&state);
    let Content::Page(host) = data.window_state(&state).content else {
        panic!("the Welcome page is shown");
    };
    let PageState::Welcome(welcome) = host.page else {
        panic!("the Welcome page is not shown");
    };
    assert_eq!(welcome.done, [true, false, false, false, false]);
    assert!(welcome.has_timeline);
}

/// The steps ticked on the Welcome page, as the page is drawn from the store.
fn steps_done(state: &State) -> [bool; 5] {
    let data = view_data(state);
    let Content::Page(host) = data.window_state(state).content else {
        panic!("the Welcome page is not shown");
    };
    let PageState::Welcome(welcome) = host.page else {
        panic!("the Welcome page is not shown");
    };
    welcome.done
}

#[test]
fn welcome_page__welcome_in_quick_search_shows_it_again_with_the_steps_done_ticked() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let commit = super::repository_state(&work).head.commit.unwrap();
    let _page = intent(&mut state, Intent::Graph(GraphIntent::Open(commit)));
    let _back = intent(&mut state, Intent::ShowTimeline);
    let _typed = intent(&mut state, Intent::Search(SearchIntent::OpenQuick(String::from("wel"))));
    let _chosen = intent(
        &mut state,
        Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(QuickCommand::Welcome))),
    );
    assert_eq!(page_shown(&state), Some((Page::Welcome, true)));
    assert_eq!(steps_done(&state), [true, false, true, false, false]);
}

#[test]
fn welcome_page__opening_a_commit_page_ticks_its_step_once_and_writes_it_down() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let commit = super::repository_state(&work).head.commit.unwrap();
    let effects = intent(&mut state, Intent::Graph(GraphIntent::Open(commit.clone())));
    assert!(state.persisted.welcome_steps[2]);
    assert!(effects.contains(&Effect::WritePersisted(state.persisted.clone())));
    let _closed = intent(&mut state, Intent::ClosePage);
    let again = intent(&mut state, Intent::Graph(GraphIntent::Open(commit)));
    assert!(
        !again.iter().any(|effect| matches!(effect, Effect::WritePersisted(_))),
        "a step ticked stays ticked and is written once"
    );
}

#[test]
fn welcome_page__a_workspace_opened_from_the_page_is_listed_first_the_next_time() {
    let (work, other) = (folder("work"), folder("other"));
    let mut state = window();
    state.persisted.recent = vec![
        RecentWorkspace {
            folders: vec![work.clone()],
            repository: None,
        },
        RecentWorkspace {
            folders: vec![other.clone()],
            repository: None,
        },
    ];
    let _shown = intent(&mut state, Intent::ShowPage(Page::Welcome));
    let effects = intent(&mut state, Intent::OpenRecent(1));
    let _read = answer(&mut state, effects, &[(&other, std::slice::from_ref(&other))]);
    let mut next = State::new(state.persisted.clone(), Settings::default(), None);
    let _started = reduce(&mut next, Message::Started { folders: Vec::new() });
    let _shown = intent(&mut next, Intent::ShowPage(Page::Welcome));
    let data = view_data(&next);
    let Content::Page(host) = data.window_state(&next).content else {
        panic!("the Welcome page is not shown");
    };
    let PageState::Welcome(welcome) = host.page else {
        panic!("the Welcome page is not shown");
    };
    assert_eq!(welcome.recent.first(), Some(&other.display().to_string()));
}

#[test]
fn quick_search_opens_the_command_log_and_the_settings_page() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    for command in [QuickCommand::ShowCommandLog, QuickCommand::OpenSettings] {
        assert!(command.is_built());
        let _chosen = intent(
            &mut state,
            Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(command))),
        );
    }
    assert!(state.is_activity_open && state.is_log_shown);
    assert_eq!(page_shown(&state), Some((Page::Settings, true)));
}

#[test]
fn welcome_page__ctrl_tab_goes_through_the_timeline_and_the_pages_and_close_comes_back() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let _shown = intent(&mut state, Intent::ShowPage(Page::Welcome));
    let _timeline = intent(&mut state, Intent::NextPage);
    assert_eq!(page_shown(&state), None);
    let _page = intent(&mut state, Intent::NextPage);
    assert_eq!(page_shown(&state), Some((Page::Welcome, true)));
    let _closed = intent(&mut state, Intent::ClosePage);
    assert_eq!(page_shown(&state), None);
    assert_eq!(state.pages.open.len(), 0);
}

#[test]
fn repo_no_repository__a_folder_with_no_repository_says_so_and_check_again_finds_a_new_one() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[]);
    let data = view_data(&state);
    assert_eq!(
        data.window_state(&state).content,
        Content::Timeline(TimelineState::NoRepository(NoRepositoryState::NoRepository))
    );
    let effects = intent(&mut state, Intent::CheckAgain);
    let data = view_data(&state);
    assert_eq!(
        data.window_state(&state).content,
        Content::Timeline(TimelineState::NoRepository(NoRepositoryState::Loading))
    );
    let effects = answer(&mut state, effects, &[(&work, std::slice::from_ref(&work))]);
    assert_eq!(reads(&effects), std::slice::from_ref(&work));
    assert!(matches!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(Shown::Read(_))
    ));
}

#[test]
fn repo_no_repository__a_read_that_fails_shows_it_and_keeps_what_git_said() {
    let mut state = window();
    let work = folder("work");
    let _opened = open(&mut state, &work, &[(&work, std::slice::from_ref(&work))]);
    let effects = intent(&mut state, Intent::Refresh);
    let Some(Effect::Start(find)) = effects.last() else {
        panic!("the repositories are looked for again");
    };
    let effects = reduce(
        &mut state,
        Message::RepositoriesFound {
            job: find.id,
            found: crate::store::Found::Repositories(vec![model::repo::Repository {
                folder: work.clone(),
                superproject: None,
            }]),
        },
    );
    let Some(Effect::Start(read)) = effects.iter().find(|effect| matches!(effect, Effect::Start(_))) else {
        panic!("the repository shown is read again");
    };
    assert!(matches!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(Shown::Read(_))
    ));
    let _failed = reduce(
        &mut state,
        Message::RepositoryRead {
            job: read.id,
            folder: work,
            result: Err(String::from("fatal: not a git repository")),
            read_at: 0,
        },
    );
    assert_eq!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(&Shown::NoRepository)
    );
    assert_eq!(state.last_error.as_deref(), Some("fatal: not a git repository"));
}

#[test]
fn settings_theme__settings_read_again_with_another_theme_redraw_the_window() {
    let mut state = window();
    let same = reduce(&mut state, Message::SettingsRead(Settings::default()));
    assert_eq!(same.len(), 0);
    let dark = Settings {
        theme: ThemeName::Dark,
        ..Settings::default()
    };
    assert_eq!(reduce(&mut state, Message::SettingsRead(dark)), [Effect::ApplySettings]);
    assert_eq!(state.settings.theme, ThemeName::Dark);
}

#[test]
fn the_window_closed_keeps_its_size_cancels_its_jobs_and_quits() {
    let mut state = window();
    let work = folder("work");
    let effects = reduce(&mut state, Message::FolderChosen(Some(work)));
    let job = super::started(&effects)[0].0;
    let effects = reduce(
        &mut state,
        Message::CloseRequested {
            window_size: Some((1000.0, 700.0)),
        },
    );
    assert_eq!(effects.first(), Some(&Effect::Cancel(job)));
    assert_eq!(effects.last(), Some(&Effect::Quit));
    assert_eq!(state.persisted.window_size, Some((1000.0, 700.0)));
}

#[test]
fn the_lists_kept_aside_and_the_activity_drawer_open_and_close() {
    let mut state = window();
    let _open = intent(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Tags)));
    let _drawer = intent(&mut state, Intent::OpenActivity);
    assert!(state.rail.lists.is_open(&RailList::Tags));
    assert!(state.is_activity_open);
    let _closed = intent(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Tags)));
    let _drawer = intent(&mut state, Intent::CloseActivity);
    assert!(!state.rail.lists.is_open(&RailList::Tags));
    assert!(!state.is_activity_open);
}
