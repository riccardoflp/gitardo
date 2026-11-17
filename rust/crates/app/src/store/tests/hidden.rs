//! `repo.watch-hidden`: the tabs not shown are watched, marked out of date by their changes, read again once shown, and
//! read one after the other in the background once the tab shown has its first read.

use std::path::{Path, PathBuf};
use std::time::Duration;

use git::read::graph::CommitPage;
use model::contract::{FeatureId, Target};
use model::repo::{Change, ReadCause};
use views::feature::FeatureIntent;
use views::{Content, Intent, TimelineState};

use super::{answer, folder, intent, open, reads, repository_state, started, view_data, window};
use crate::store::feature::Ended;
use crate::store::{Effect, Found, JobId, JobKind, Message, State, reduce};

/// The folders `api`, `web` and `docs` of `work`.
fn three_folders() -> (PathBuf, Vec<PathBuf>) {
    let work = folder("work");
    let repositories = ["api", "web", "docs"].iter().map(|name| work.join(name)).collect();
    (work, repositories)
}

/// A window showing `api`, `web` and `docs`, each in its tab, `api` shown, and each read.
fn three_tabs() -> (State, Vec<PathBuf>) {
    let mut state = window();
    let (work, repositories) = three_folders();
    let _opened = open(&mut state, &work, &[(&work, &repositories)]);
    (state, repositories)
}

/// A commit made in a terminal, as the watcher of a tab sees it.
fn commit_in_a_terminal(folder: &Path) -> Message {
    Message::TabChanged {
        folder: folder.to_path_buf(),
        changes: vec![Change::GitFolder(PathBuf::from("refs/heads/main"))],
        at: Duration::from_secs(5),
    }
}

/// The reads of repositories an answer started, with their jobs.
fn read_jobs(effects: &[Effect]) -> Vec<(JobId, PathBuf)> {
    started(effects)
        .into_iter()
        .filter_map(|(job, kind)| match kind {
            JobKind::ReadRepository { folder, .. } => Some((job, folder)),
            _ => None,
        })
        .collect()
}

/// A read of a repository answered as a worker would.
fn read(state: &mut State, (job, folder): (JobId, PathBuf)) -> Vec<Effect> {
    reduce(
        state,
        Message::RepositoryRead {
            job,
            result: Ok(Box::new((
                repository_state(&folder),
                Some(CommitPage::default()),
                git::read::worktree::CountedWorktrees::default(),
            ))),
            folder,
            read_at: 0,
        },
    )
}

/// Opens the folder `work` with some repositories, all of them in tabs, and gives the read the window starts.
fn opening(state: &mut State, work: &Path, repositories: &[PathBuf]) -> Vec<Effect> {
    let effects = reduce(state, Message::FolderChosen(Some(work.to_path_buf())));
    let (job, _) = started(&effects).into_iter().next().unwrap();
    let found = repositories
        .iter()
        .map(|folder| model::repo::Repository {
            folder: folder.clone(),
            superproject: None,
            common_folder: None,
        })
        .collect();
    let _asked = reduce(
        state,
        Message::RepositoriesFound {
            job,
            found: Found::Repositories(found),
        },
    );
    intent(state, Intent::TabsChosen(Some((0..repositories.len()).collect())))
}

/// Whether the timeline shows a repository read, and not **Loading repository…**.
fn shows_a_repository(state: &State) -> bool {
    let data = view_data(state);
    matches!(
        data.window_state(state).content,
        Content::Timeline(TimelineState::Repository { .. })
    )
}

#[test]
fn repo_watch_hidden__a_commit_in_a_terminal_in_web_while_api_is_shown_reads_nothing_and_one_read_follows_showing_it() {
    let (mut state, repositories) = three_tabs();
    let web = repositories[1].clone();
    let effects = reduce(&mut state, commit_in_a_terminal(&web));
    assert!(reads(&effects).is_empty(), "nothing is read");
    let cause = ReadCause::GitFile(PathBuf::from("refs/heads/main"));
    let kept = state.tabs.open[1].kept.as_ref().unwrap();
    assert_eq!(kept.hidden.out_of_date(), Some(&cause));
    let effects = intent(&mut state, Intent::SwitchTab(1));
    assert!(shows_a_repository(&state), "what it kept, at once");
    let jobs = read_jobs(&effects);
    assert_eq!(jobs.iter().map(|(_, folder)| folder.clone()).collect::<Vec<_>>(), [web]);
    assert_eq!(
        state.reads.reason(jobs[0].0).unwrap().cause,
        cause,
        "the activity indicator says why"
    );
    let all = answer(&mut state, effects, &[]);
    assert_eq!(read_jobs(&all).len(), 1, "one read, and none after it");
}

#[test]
fn repo_watch_hidden__eight_tabs_read_the_tab_shown_first_then_the_seven_others_one_at_a_time() {
    let mut state = window();
    let work = folder("work");
    let repositories: Vec<PathBuf> = ["api", "web", "docs", "app", "lib", "site", "cli", "tools"]
        .iter()
        .map(|name| work.join(name))
        .collect();
    let mut effects = opening(&mut state, &work, &repositories);
    let mut order = Vec::new();
    loop {
        let jobs = read_jobs(&effects);
        let Some(next) = jobs.first().cloned() else {
            break;
        };
        assert_eq!(jobs.len(), 1, "one read at a time");
        order.push(next.1.clone());
        effects = read(&mut state, next);
    }
    assert_eq!(order, repositories);
    assert!(state.tabs.open[1..].iter().all(|tab| tab.kept.is_some()));
}

#[test]
fn repo_watch_hidden__a_tab_not_shown_in_which_nothing_changed_is_shown_with_no_read() {
    let (mut state, repositories) = three_tabs();
    let effects = intent(&mut state, Intent::SwitchTab(2));
    assert!(reads(&effects).is_empty(), "showing docs reads nothing");
    assert!(shows_a_repository(&state));
    assert!(state.shown_folder().is_some_and(|shown| shown == &repositories[2]));
    let effects = intent(&mut state, Intent::SwitchTab(0));
    assert!(reads(&effects).is_empty(), "nor coming back to api");
}

#[test]
fn repo_watch_hidden__while_an_operation_runs_in_api_no_other_tab_is_read() {
    let mut state = window();
    let (work, repositories) = three_folders();
    let effects = opening(&mut state, &work, &repositories);
    let api = read_jobs(&effects)[0].clone();
    let effects = read(&mut state, api);
    let web = read_jobs(&effects)[0].clone();
    assert_eq!(web.1, repositories[1], "web is read in the background");
    let run = FeatureIntent::Run(FeatureId("branch.create"), Target::Head);
    let effects = intent(&mut state, Intent::Feature(run));
    assert!(state.is_operation_running);
    assert!(
        effects.contains(&Effect::Cancel(web.0)),
        "the read of web waits for the operation"
    );
    assert!(reads(&effects).is_empty(), "nothing is read while it runs");
    assert!(state.tabs.open[1].kept.is_none(), "web waits for its turn again");
    let effects = read(&mut state, web);
    assert!(reads(&effects).is_empty(), "what the read stopped found is dropped");
    let effects = reduce(
        &mut state,
        Message::FeatureEnded {
            ended: Ended::BackedOut,
            log: Vec::new(),
        },
    );
    assert_eq!(reads(&effects), [repositories[1].clone()], "once it ends, web is read");
}

#[test]
fn the_tabs_not_shown_are_watched_with_the_setting_on_and_not_with_it_off() {
    let mut state = window();
    let (work, repositories) = three_folders();
    let effects = open(&mut state, &work, &[(&work, &repositories)]);
    let watched: Vec<&Vec<PathBuf>> = effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::WatchTabs(folders) => Some(folders),
            _ => None,
        })
        .collect();
    assert_eq!(watched.last().unwrap().as_slice(), &repositories[1..]);
    let mut settings = state.settings.clone();
    settings.keep_open = false;
    let effects = reduce(&mut state, Message::SettingsRead(Box::new(settings)));
    assert!(effects.contains(&Effect::WatchTabs(Vec::new())));
}
