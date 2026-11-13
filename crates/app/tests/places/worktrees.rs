//! The worktrees in the window: another worktree shown in it with its banner, going back, the buttons offered once a
//! worktree is made or could not be removed, and a new window asked for.

use std::path::{Path, PathBuf};

use app::store::{Effect, Message, State};
use model::contract::{Answer, Answers, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;

use super::showing;
use super::support::{OperationsDriver, Window, has, run, says};

/// The size of the window.
const WIDE: (u32, u32) = (1000, 520);

/// A repository with a commit, and a worktree of `feature/login` in the folder of the worktrees beside it.
fn with_worktree() -> (Repository, PathBuf) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let folder = repository
        .path()
        .with_file_name("repository.worktrees")
        .join("feature/login");
    let _ = repository.git(&["worktree", "add", "-b", "feature/login", &folder.to_string_lossy()]);
    (repository, folder)
}

/// Runs an operation from the window with some answers given, and waits until it ended.
fn run_operation(state: &mut State, feature: &'static str, target: Target, answers: Answers) -> OperationsDriver {
    let driver = OperationsDriver::new();
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId(feature), target, answers)),
    );
    while !matches!(driver.next(state), Message::FeatureEnded { .. }) {}
    driver
}

/// The folder the views show.
fn shown(state: &State) -> PathBuf {
    state.shown_folder().cloned().unwrap_or_default()
}

/// Whether two folders are the same, whatever the slashes and the case.
fn same(left: &Path, right: &Path) -> bool {
    model::worktree::is_listed(&[left.to_path_buf()], right)
}

/// The same repository shown in this window, on the worktree of `feature/login`.
fn showing_login() -> (Repository, PathBuf, State) {
    let (repository, folder) = with_worktree();
    let mut state = showing(repository.path().to_path_buf());
    let _driver = run_operation(
        &mut state,
        "worktree.show-in-gitardo",
        Target::Worktree(folder.clone()),
        Answers::new(),
    );
    (repository, folder, state)
}

#[test]
fn worktree_show_in_gitardo__a_commit_made_from_the_draft_is_made_in_that_worktree() {
    let (repository, folder) = with_worktree();
    std::fs::write(folder.join("login.txt"), "sign in\n").unwrap();
    let _ = repository.git(&["-C", &folder.to_string_lossy(), "add", "login.txt"]);
    let mut state = showing(repository.path().to_path_buf());
    let target = Target::Worktree(folder.clone());
    let _shown = run_operation(&mut state, "worktree.show-in-gitardo", target, Answers::new());
    assert!(same(&shown(&state), &folder), "{:?}", shown(&state));
    let main_before = repository.git(&["rev-parse", "HEAD"]);
    let message = Answers::new().with(features::commit::MESSAGE, Answer::Text(String::from("Sign in")));
    let _committed = run_operation(&mut state, "commit.create", Target::Head, message);
    let made = repository.git(&["log", "-1", "--format=%s", "feature/login"]);
    assert_eq!(made.trim(), "Sign in");
    assert_eq!(
        repository.git(&["rev-parse", "HEAD"]),
        main_before,
        "the window's folder is untouched"
    );
}

#[test]
fn worktree_show_in_gitardo__opening_the_window_again_shows_its_own_folder() {
    let (repository, _folder, state) = showing_login();
    let remembered = state
        .persisted
        .recent
        .first()
        .and_then(|recent| recent.repository.clone());
    assert!(
        remembered
            .as_deref()
            .is_none_or(|remembered| same(remembered, repository.path())),
        "{remembered:?}"
    );
}

#[test]
fn worktree_banner__it_says_the_worktree_shown_above_the_timeline_and_on_the_changes_page() {
    let (_repository, _folder, mut state) = showing_login();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Showing the worktree feature/login on feature/login"));
    let _effects = run(
        &mut state,
        Message::Intent(Intent::ShowPage(views::page::Page::Changes)),
    );
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Showing the worktree feature/login on feature/login"));
}

#[test]
fn worktree_banner__back_to_app_shows_the_folder_of_the_window_again_and_the_banner_goes() {
    let (repository, _folder, mut state) = showing_login();
    let mut window = Window::new(ThemeName::Dark, WIDE);
    for intent in window.click(&state, Role::Button, "Back to repository") {
        let _effects = run(&mut state, Message::Intent(intent));
    }
    assert!(same(&shown(&state), repository.path()), "{:?}", shown(&state));
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, "Showing the worktree"));
}

#[test]
fn worktree_banner__no_banner_shows_while_the_window_shows_its_own_folder() {
    let (repository, _folder) = with_worktree();
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, "Showing the worktree"));
}

#[test]
fn worktree_create__once_made_its_offers_are_buttons_that_open_it() {
    let repository = Repository::new();
    let mut repository = repository;
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let mut state = showing(repository.path().to_path_buf());
    let answers = Answers::new()
        .with(features::worktree::START, Answer::Choice(String::from("main")))
        .with(features::worktree::NAME, Answer::Text(String::from("fix")));
    let _driver = run_operation(&mut state, "worktree.create", Target::Head, answers);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Open in new window"));
    assert!(
        has(&tree, Role::Button, "Choose What It Gets\u{2026}"),
        "the first time"
    );
    let intents = window.click(&state, Role::Button, "Open in new window");
    let Some(Intent::Feature(FeatureIntent::RunWith(feature, Target::Worktree(folder), _))) = intents.first() else {
        panic!("{intents:?}")
    };
    assert_eq!(*feature, FeatureId("worktree.open-in-new-window"));
    let effects = run(&mut state, Message::Intent(intents.first().cloned().unwrap()));
    let driver = OperationsDriver::new();
    driver.take(&state, &effects);
    let ended = driver.next(&mut state);
    assert!(matches!(ended, Message::FeatureEnded { .. }), "{ended:?}");
    assert!(state.worktrees.setup_offered, "Choose What It Gets… is offered once");
    assert!(folder.ends_with("fix"), "{folder:?}");
}

#[test]
fn worktree_open_in_new_window__the_window_asks_for_a_new_window_and_stays_as_it_was() {
    let (repository, folder) = with_worktree();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let effects = run(
        &mut state,
        Message::Intent(Intent::Feature(FeatureIntent::Run(
            FeatureId("worktree.open-in-new-window"),
            Target::Worktree(folder.clone()),
        ))),
    );
    driver.take(&state, &effects);
    let ended = loop {
        let message = driver_message(&driver);
        if let Message::FeatureEnded { .. } = &message {
            break message;
        }
    };
    let effects = run(&mut state, ended);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::OpenWindow(opened) if same(opened, &folder))),
        "{effects:?}"
    );
    assert!(same(&shown(&state), repository.path()));
}

#[test]
fn worktree_remove__a_worktree_with_a_modified_file_offers_remove_anyway() {
    let (repository, folder) = with_worktree();
    std::fs::write(folder.join("README.md"), "changed\n").unwrap();
    let mut state = showing(repository.path().to_path_buf());
    let answers = Answers::new().with(
        model::worktree::REMOVE_BRANCH,
        Answer::Choice(model::worktree::KEEP_BRANCH.to_owned()),
    );
    state.settings.features.command_preview = model::contract::CommandPreview::Off;
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::RunWith(
            FeatureId("worktree.remove"),
            Target::Worktree(folder.clone()),
            answers,
        )),
    );
    loop {
        match driver.next(&mut state) {
            Message::FeatureEnded { .. } => break,
            Message::FeatureConfirms(_) => driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(true))),
            _ => {}
        }
    }
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "Remove anyway"));
    assert!(
        folder.join("README.md").is_file(),
        "nothing is deleted until it is accepted"
    );
}

/// What the thread of the operations says next, not reduced.
fn driver_message(driver: &OperationsDriver) -> Message {
    driver.raw_next()
}

#[test]
fn worktree_show_in_graph__a_worktree_on_a_commit_far_down_the_history_is_loaded_and_selected() {
    let repository = super::histories::imported(&super::histories::line(150));
    let first = repository.git(&["rev-list", "--max-parents=0", "HEAD"]);
    let commit = model::CommitId::parse(first.trim_end()).unwrap();
    let folder = repository.path().with_file_name("repository.worktrees").join("first");
    let _ = repository.git(&[
        "worktree",
        "add",
        "--detach",
        &folder.to_string_lossy(),
        first.trim_end(),
    ]);
    let mut state = super::window();
    state.settings.page_size = model::graph::PageSize::from_setting(Some(100.0));
    let folders = vec![repository.path().to_path_buf()];
    let _effects = run(&mut state, Message::Started { folders });
    assert!(!state.timeline.layout.shown_of.contains_key(&commit), "not loaded yet");
    let _driver = run_operation(
        &mut state,
        "worktree.show-in-graph",
        Target::Worktree(folder),
        Answers::new(),
    );
    assert!(state.timeline.layout.shown_of.contains_key(&commit), "loaded");
    assert_eq!(state.timeline.selected, Some(views::graph::Selected::Commit(commit)));
}

