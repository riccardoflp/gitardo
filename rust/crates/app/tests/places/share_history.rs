//! A worktree shown for the first time takes the history its repository has read (`repo.share-history`), over a
//! repository `api` of three commits on `main` with a worktree `api-fix` on the branch `fix`, one commit ahead.

use std::fs;
use std::path::{Path, PathBuf};

use app::persisted::same_folder;
use app::store::{Effect, Job, JobKind, Message, Shown, State, reduce};
use app::work::jobs;
use git::CancelToken;
use model::filter::show_only;
use model::graph::Row;
use testkit::Repository;
use views::{FilterIntent, Intent};

use super::support::run;

/// `api` and the folder of its worktree `api-fix`, which has one file not committed.
fn api_with_fix() -> (Repository, PathBuf) {
    let mut api = Repository::new();
    let _ = api.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = api.commit("Read the settings at startup", &[("settings.txt", "theme\n")]);
    let _ = api.commit("Show the branch rail", &[("rail.txt", "branches\n")]);
    let fix = api.path().parent().unwrap().join("api-fix");
    let _ = api.git(&["worktree", "add", "--quiet", "-b", "fix", fix.to_str().unwrap()]);
    commit_in(&api, &fix, "Fix the parser", "parser.rs");
    fs::write(fix.join("notes.txt"), "half done\n").unwrap();
    (api, fix)
}

/// Makes a commit of one new file in a worktree of `api`.
fn commit_in(api: &Repository, worktree: &Path, message: &str, file: &str) {
    fs::write(worktree.join(file), "1\n").unwrap();
    let folder = worktree.to_str().unwrap();
    let _ = api.git(&["-C", folder, "add", file]);
    let _ = api.git(&["-C", folder, "commit", "--quiet", "-m", message]);
}

/// A window on `api` and its worktree, `api` shown and read.
fn window(api: &Repository, fix: &Path) -> State {
    let mut state = super::window();
    let folders = vec![api.path().to_path_buf(), fix.to_path_buf()];
    let _ = run(&mut state, Message::Started { folders });
    assert!(shows(&state, api.path()));
    state
}

/// Whether the repository shown is the one of a folder.
fn shows(state: &State, folder: &Path) -> bool {
    state.shown_folder().is_some_and(|shown| same_folder(shown, folder))
}

/// The subjects of the commits of the timeline, from the top.
fn subjects(state: &State) -> Vec<String> {
    let commits = state.timeline.rows.iter().filter_map(Row::commit);
    commits.map(|commit| commit.subject.clone()).collect()
}

/// Chooses a member of the tab and gives the read it starts, not run yet: the window shows what it shows at once.
fn choose(state: &mut State, folder: &Path) -> Job {
    let effects = reduce(state, Message::Intent(Intent::ShowInTab(0, folder.to_path_buf())));
    let read = effects.iter().find_map(|effect| match effect {
        Effect::Start(job) if matches!(job.kind, JobKind::ReadRepository { .. }) => Some(job.clone()),
        _ => None,
    });
    read.unwrap()
}

/// Runs a read as a worker would and gives the window what it sends; says whether `git log` ran, a page of commits
/// having come with the first step.
fn read(state: &mut State, job: &Job) -> bool {
    let messages = jobs::messages(job, &CancelToken::new());
    let has_page = match messages.first() {
        Some(Message::FirstRead { result: Ok(read), .. }) => read.1.is_some(),
        first => panic!("the first step comes first: {first:?}"),
    };
    for message in messages {
        let _effects = run(state, message);
    }
    has_page
}

/// Whether a read takes the rows of the view left, to read the commits only when the history is another.
fn takes_rows(job: &Job) -> bool {
    matches!(
        &job.kind,
        JobKind::ReadRepository {
            kept: Some(_),
            in_steps: true,
            ..
        }
    )
}

/// The branch checked out, as the window has it.
fn branch(state: &State) -> Option<String> {
    state.shown_read().and_then(|read| read.state.head.branch.clone())
}

#[test]
fn repo_share_history__a_worktree_chosen_shows_the_rows_of_api_with_no_git_log_then_what_it_has_checked_out() {
    let (api, fix) = api_with_fix();
    let mut state = window(&api, &fix);
    let rows = subjects(&state);
    assert_eq!(rows.len(), 4);
    let job = choose(&mut state, &fix);
    assert!(shows(&state, &fix) && takes_rows(&job));
    assert_eq!(subjects(&state), rows, "the same rows, at once");
    assert_eq!(
        branch(&state).as_deref(),
        Some("refs/heads/main"),
        "until its first step is read"
    );
    assert!(!read(&mut state, &job), "no git log ran");
    assert_eq!(subjects(&state), rows);
    assert_eq!(branch(&state).as_deref(), Some("refs/heads/fix"));
    assert!(
        matches!(state.timeline.rows.first(), Some(Row::Uncommitted { files: 1, .. })),
        "the uncommitted changes of api-fix: {:?}",
        state.timeline.rows.first()
    );
    let other = state.shown_read().unwrap().state.worktrees.clone();
    assert!(
        other.iter().all(|worktree| worktree.uncommitted.is_some()),
        "the rest came: {other:?}"
    );
}

#[test]
fn repo_share_history__choosing_back_api_with_nothing_kept_takes_the_rows_of_its_worktree_the_same_way() {
    let (api, fix) = api_with_fix();
    let mut state = window(&api, &fix);
    state.settings.keep_open = false;
    let rows = subjects(&state);
    let job = choose(&mut state, &fix);
    let _ = read(&mut state, &job);
    let job = choose(&mut state, api.path());
    assert!(shows(&state, api.path()) && takes_rows(&job));
    assert_eq!(subjects(&state), rows, "the rows of api-fix, at once");
    assert!(!read(&mut state, &job), "no git log ran");
    assert_eq!(subjects(&state), rows);
    assert_eq!(branch(&state).as_deref(), Some("refs/heads/main"));
    assert!(
        !matches!(state.timeline.rows.first(), Some(Row::Uncommitted { .. })),
        "api has nothing to commit"
    );
}

#[test]
fn repo_share_history__a_worktree_detached_at_a_commit_no_reference_reaches_shows_its_commit_above_after_one_git_log() {
    let (api, fix) = api_with_fix();
    let _ = api.git(&["-C", fix.to_str().unwrap(), "checkout", "--quiet", "--detach"]);
    commit_in(&api, &fix, "Only in the worktree", "only.rs");
    let mut state = window(&api, &fix);
    let rows = subjects(&state);
    assert!(!rows.contains(&String::from("Only in the worktree")));
    let job = choose(&mut state, &fix);
    assert!(takes_rows(&job));
    assert_eq!(subjects(&state), rows, "the rows of its repository, at once");
    assert!(read(&mut state, &job), "one git log ran");
    let after = subjects(&state);
    assert_eq!(after.first().map(String::as_str), Some("Only in the worktree"));
    assert_eq!(after.get(1..), Some(rows.as_slice()), "its own commit above them");
}

#[test]
fn repo_share_history__a_worktree_takes_the_rows_of_main_only_when_it_is_narrowed_to_main_too() {
    let (api, fix) = api_with_fix();
    let mut state = window(&api, &fix);
    state.settings.keep_open = false;
    let narrow = || Message::Intent(Intent::Filter(FilterIntent::Show(Some(show_only("refs/heads/main")))));
    let _ = run(&mut state, narrow());
    let of_main = subjects(&state);
    assert_eq!(of_main.len(), 3, "api is narrowed to main: {of_main:?}");
    let job = choose(&mut state, &fix);
    assert!(
        !takes_rows(&job),
        "api-fix shows every branch: it is read as a repository chosen"
    );
    assert!(matches!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(Shown::Reading { .. })
    ));
    assert!(read(&mut state, &job));
    assert_eq!(subjects(&state).len(), 4);
    let _ = run(&mut state, narrow());
    assert_eq!(subjects(&state), of_main);
    let job = choose(&mut state, api.path());
    assert!(takes_rows(&job), "both are narrowed to main");
    assert_eq!(subjects(&state), of_main, "the rows of main, at once");
    assert!(!read(&mut state, &job), "no git log ran");
    assert_eq!(subjects(&state), of_main);
}

#[test]
fn repo_share_history__a_submodule_of_api_chosen_takes_no_row_of_api() {
    let repositories = super::group_tabs::made(true);
    let mut state = super::group_tabs::window(&repositories);
    assert!(!state.timeline.rows.is_empty(), "api is read");
    let submodule = repositories.api.join("lib").join("core");
    let job = choose(&mut state, &submodule);
    assert!(!takes_rows(&job));
    assert!(matches!(
        state.workspace.as_ref().map(|workspace| &workspace.shown),
        Some(Shown::Reading { .. })
    ));
    assert!(read(&mut state, &job), "its own commits are read");
    assert_eq!(subjects(&state), ["Add the core"]);
}
