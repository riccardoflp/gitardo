//! The Stashes list of the branch rail drawn from the store, with repositories made for each test (`stash.pane`): its
//! title, a stash unfolded to its files, the Diff page a file opens, the buttons of the rows, the list with no stash,
//! a stash dropped from elsewhere, **Show Stashes** in Quick search, and the warning of a pull that left its autostash
//! (`stash.autostash-left`).

use app::store::{Message, State};
use model::CommitId;
use model::contract::{FeatureId, Target};
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::{Remote, Repository};
use views::diff::{DiffFile, DiffIntent, DiffTarget, DiffVersion};
use views::feature::FeatureIntent;
use views::search::{QuickChoice, QuickCommand, SearchIntent};
use views::text::stash as words;
use views::{Intent, RailIntent, RailList};

use super::pointer::{bounds, middle, moved};
use super::showing;
use super::support::{OperationsDriver, Window, has, run, says};

/// The size of the window of these tests.
const WIDE: (u32, u32) = (1000, 580);

/// A repository whose stash changed `a.txt` and `b.txt` and took the untracked `new.txt`, and the commit it was made on.
fn stashed_with_untracked() -> (Repository, String) {
    let mut repository = Repository::new();
    let base = repository.commit("Add a and b", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    for (name, text) in [("a.txt", "a, mine\n"), ("b.txt", "b, mine\n"), ("new.txt", "new\n")] {
        std::fs::write(repository.path().join(name), text).unwrap();
    }
    let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked", "-m", "with new"]);
    (repository, base)
}

/// The commit a revision of a repository names.
fn commit_of(repository: &Repository, revision: &str) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", revision]).trim()).unwrap()
}

/// Hands an intent to the store, running the jobs it starts.
fn ask(state: &mut State, intent: Intent) {
    let _effects = run(state, Message::Intent(intent));
}

/// A window showing a repository, with the Stashes list open.
fn with_stashes_open(repository: &Repository) -> State {
    let mut state = showing(repository.path().to_path_buf());
    ask(&mut state, Intent::Rail(RailIntent::Toggle(RailList::Stashes)));
    state
}

/// The top of a widget of a role and a name.
fn top_of(tree: &TreeUpdate, role: Role, name: &str) -> f32 {
    bounds(tree, role, name).y
}

/// The window drawn with the pointer resting on the row of a stash, by its message.
fn pointing_at(window: &mut Window, state: &State, message: &str) -> TreeUpdate {
    let (_, tree) = window.draw(state, &[]);
    let row = bounds(&tree, Role::Button, message);
    let (_, _) = window.draw(state, &[moved(middle(row))]);
    window.draw(state, &[]).1
}

#[test]
fn stash_pane__the_title_of_the_list_says_how_many_stashes_there_are() {
    let (repository, _) = stashed_with_untracked();
    std::fs::write(repository.path().join("a.txt"), "a, again\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "-m", "again"]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    let title = super::support::find(&tree, Role::Button, views::text::rail::STASHES);
    assert_eq!(title.value(), Some("2"));
}

#[test]
fn stash_pane__unfolding_a_stash_with_untracked_files_lists_its_changed_files_then_its_untracked_ones() {
    let (repository, _) = stashed_with_untracked();
    let mut state = with_stashes_open(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let unfold = window.click(&state, Role::Button, words::UNFOLD);
    let toggled = commit_of(&repository, "stash@{0}");
    assert_eq!(unfold, [Intent::Rail(RailIntent::ToggleStash(toggled))]);
    for intent in unfold {
        ask(&mut state, intent);
    }
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, words::FOLD), "the arrow folds it now");
    let tops: Vec<f32> = ["a.txt", "b.txt", "new.txt"]
        .iter()
        .map(|name| top_of(&tree, Role::Button, name))
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "{tops:?}");
    assert!(top_of(&tree, Role::Button, "On main: with new") < tops[0]);
}

#[test]
fn stash_pane__a_click_on_a_changed_file_opens_the_diff_page_between_the_commit_it_was_made_on_and_the_stash() {
    let (repository, base) = stashed_with_untracked();
    let mut state = with_stashes_open(&repository);
    let stash = commit_of(&repository, "stash@{0}");
    ask(&mut state, Intent::Rail(RailIntent::ToggleStash(stash.clone())));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let clicked = window.click(&state, Role::Button, "b.txt");
    let target = DiffTarget::file(
        DiffVersion::commit(CommitId::parse(base.trim()).unwrap()),
        DiffVersion::commit(stash),
        DiffFile::same("b.txt"),
    );
    assert_eq!(clicked, [Intent::Diff(DiffIntent::Open(Box::new(target)))]);
}

#[test]
fn stash_pane__the_buttons_of_the_rows_are_not_there_while_an_operation_runs() {
    let (repository, _) = stashed_with_untracked();
    let mut state = with_stashes_open(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let tree = pointing_at(&mut window, &state, "On main: with new");
    for button in [words::APPLY, words::POP, words::DROP] {
        assert!(has(&tree, Role::Button, button), "{button} is there under the pointer");
    }
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(FeatureId("stash.drop"), Target::Stash(0))),
    );
    while !matches!(driver.next(&mut state), Message::FeatureConfirms(_)) {}
    assert!(state.is_operation_running, "the drop waits to be confirmed");
    let tree = pointing_at(&mut window, &state, "On main: with new");
    for button in [words::APPLY, words::POP, words::DROP] {
        assert!(
            !has(&tree, Role::Button, button),
            "{button} is not there while the drop runs"
        );
    }
    driver.intent(&mut state, Intent::Feature(FeatureIntent::Confirm(false)));
    while !matches!(driver.next(&mut state), Message::FeatureEnded { .. }) {}
    assert_eq!(
        repository.git(&["stash", "list", "--format=%gs"]).trim(),
        "On main: with new"
    );
}

#[test]
fn stash_pane__with_no_stash_the_list_says_how_to_make_one() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
    let state = with_stashes_open(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, words::NO_STASHES));
    let clicked = window.click(&state, Role::Button, words::STASH_ALL);
    assert_eq!(
        clicked,
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("stash.push"),
            Target::Head
        ))]
    );
}

#[test]
fn stash_pane__a_stash_dropped_from_elsewhere_leaves_the_list_when_the_repository_is_read_again() {
    let (repository, _) = stashed_with_untracked();
    let mut state = with_stashes_open(&repository);
    let stash = commit_of(&repository, "stash@{0}");
    ask(&mut state, Intent::Rail(RailIntent::ToggleStash(stash.clone())));
    assert!(state.rail.stashes.files.contains_key(&stash), "its files are read");
    let _ = repository.git(&["stash", "drop", "--quiet"]);
    ask(&mut state, Intent::Refresh);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, "On main: with new"));
    assert!(!has(&tree, Role::Button, "a.txt"));
    assert!(says(&tree, words::NO_STASHES));
    assert!(state.rail.stashes.files.is_empty() && state.rail.stashes.unfolded.is_empty());
}

#[test]
fn stash_pane__show_stashes_in_quick_search_opens_the_list() {
    let (repository, _) = stashed_with_untracked();
    let mut state = showing(repository.path().to_path_buf());
    assert!(!state.rail.lists.is_open(&RailList::Stashes));
    ask(
        &mut state,
        Intent::Search(SearchIntent::ChooseQuick(QuickChoice::Command(
            QuickCommand::ShowStashes,
        ))),
    );
    assert!(state.rail.lists.is_open(&RailList::Stashes));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(has(&tree, Role::Button, "On main: with new"));
}

/// A repository whose `main` is behind `origin/main` by a commit that changes the line `a.txt` has changed in the working
/// tree, and that has a stash of its own already.
fn autostash_conflict() -> (Repository, Remote) {
    let mut repository = Repository::new();
    let base = repository.commit("Add a", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    let remote = Remote::new(&repository);
    let _theirs = repository.commit("Change a", &[("a.txt", "a, theirs\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    std::fs::write(repository.path().join("b.txt"), "b, older\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "-m", "older"]);
    std::fs::write(repository.path().join("a.txt"), "a, mine\n").unwrap();
    (repository, remote)
}

/// Pulls with `--autostash` from the window, confirming what it shows first, until it ended.
fn pull(state: &mut State) {
    let driver = OperationsDriver::new();
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::Run(FeatureId("sync.pull"), Target::Head)),
    );
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

#[test]
fn stash_autostash_left__a_pull_that_leaves_its_autostash_warns_in_the_window_with_show_stashes() {
    let (repository, _remote) = autostash_conflict();
    let mut state = showing(repository.path().to_path_buf());
    pull(&mut state);
    let messages = repository.git(&["stash", "list", "--format=%gs"]);
    assert!(messages.lines().next().unwrap().contains("autostash"), "{messages}");
    assert!(repository.git(&["status", "--porcelain"]).contains("UU a.txt"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(super::support::says_in(
        &tree,
        "Pulled, but your changes do not fit on what came in."
    ));
    assert!(has(&tree, Role::Button, words::SHOW_STASHES));
}

#[test]
fn stash_autostash_left__show_stashes_shows_the_stashes_list_with_that_stash_first() {
    let (repository, _remote) = autostash_conflict();
    let mut state = showing(repository.path().to_path_buf());
    super::fold_changes(&mut state);
    pull(&mut state);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let clicked = window.click(&state, Role::Button, words::SHOW_STASHES);
    assert_eq!(clicked, [Intent::Rail(RailIntent::ShowStashes)]);
    for intent in clicked {
        ask(&mut state, intent);
    }
    let (_, tree) = window.draw(&state, &[]);
    let autostash = state
        .shown_read()
        .unwrap()
        .state
        .stashes
        .first()
        .unwrap()
        .message
        .clone();
    assert!(autostash.contains("autostash"), "{autostash}");
    assert!(top_of(&tree, Role::Button, &autostash) < top_of(&tree, Role::Button, "On main: older"));
}

#[test]
fn the_stashes_list_with_a_stash_unfolded_and_the_pointer_on_another_is_drawn() {
    let (repository, _) = stashed_with_untracked();
    std::fs::write(repository.path().join("a.txt"), "a, again\n").unwrap();
    let _ = repository.git(&["stash", "push", "--quiet", "-m", "again"]);
    let mut state = with_stashes_open(&repository);
    ask(
        &mut state,
        Intent::Rail(RailIntent::ToggleStash(commit_of(&repository, "stash@{1}"))),
    );
    for theme in [ThemeName::Dark, ThemeName::Light] {
        let mut window = Window::new(theme, WIDE);
        let _tree = pointing_at(&mut window, &state, "On main: again");
        let name = if theme == ThemeName::Dark { "dark" } else { "light" };
        window.assert_snapshot(&format!("stashes-unfolded-{name}"));
    }
}

#[test]
fn the_stashes_list_with_no_stash_is_drawn() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
    let state = with_stashes_open(&repository);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, _) = window.draw(&state, &[]);
    window.assert_snapshot("stashes-none-dark");
}
