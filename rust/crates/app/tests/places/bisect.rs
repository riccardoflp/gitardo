//! A bisect from the window: started and marked from the menu of a commit, answered and ended from the banner above the
//! commits, and the message that names the first bad commit.

use app::store::{Message, State};
use model::CommitId;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, Target};
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::graph::GraphIntent;
use views::text::bisect as words;

use super::showing;
use super::support::{OperationsDriver, Window, has, says};

/// The size of the window of the tests.
const WIDE: (u32, u32) = (1200, 560);

/// `main` with the commits `A` to `E`, one after the other, their hashes in that order.
fn five_commits() -> (Repository, Vec<CommitId>) {
    let mut repository = Repository::new();
    let hashes = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|name| {
            let hash = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "line\n")]);
            CommitId::parse(hash.trim()).unwrap()
        })
        .collect();
    (repository, hashes)
}

/// The commit of a letter, `A` to `E`.
fn at(hashes: &[CommitId], letter: char) -> CommitId {
    hashes
        .get(usize::from(u8::try_from(letter).unwrap() - b'A'))
        .unwrap()
        .clone()
}

/// Runs `git bisect` in the repository with some words.
fn bisect(repository: &Repository, words: &[&str]) {
    let mut arguments = vec!["bisect"];
    arguments.extend_from_slice(words);
    let _said = repository.git(&arguments);
}

/// Gives the operation thread the intents of a frame, then waits until the operation ends, confirming its plan.
fn give_until_ended(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

/// The intent of an entry of the menu of a commit that gives a mark.
fn marked(feature: &'static str, commit: &CommitId, mark: &str) -> Vec<Intent> {
    let answers = Answers::new().with(AnswerKey("mark"), Answer::Choice(mark.to_owned()));
    vec![Intent::Feature(FeatureIntent::RunWith(
        FeatureId(feature),
        Target::Commit(commit.clone()),
        answers,
    ))]
}

/// The git commands the operations ran, as the Command log has them.
fn logged(state: &State) -> Vec<String> {
    state
        .log
        .entries()
        .filter(|entry| super::support::is_ran(entry))
        .map(|entry| entry.line.clone())
        .collect()
}

/// The commit checked out.
fn head(repository: &Repository) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", "HEAD"]).trim()).unwrap()
}

#[test]
fn bisect_start__start_bad_on_e_runs_start_and_bad_and_the_banner_asks_for_a_good_commit() {
    let (repository, hashes) = five_commits();
    let e = at(&hashes, 'E');
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    give_until_ended(&driver, &mut state, marked("bisect.start", &e, "bad"));
    assert_eq!(
        logged(&state),
        ["git bisect start".to_owned(), format!("git bisect bad {e}")]
    );
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, words::MARK_GOOD_TO_BEGIN));
    assert!(has(&tree, Role::Button, words::END));
}

#[test]
fn bisect_mark__with_e_marked_bad_marking_a_good_checks_out_a_commit_between_them() {
    let (repository, hashes) = five_commits();
    let (a, e) = (at(&hashes, 'A'), at(&hashes, 'E'));
    bisect(&repository, &["start"]);
    bisect(&repository, &["bad", e.as_str()]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    give_until_ended(&driver, &mut state, marked("bisect.mark", &a, "good"));
    assert_eq!(logged(&state), [format!("git bisect good {a}")]);
    let between = head(&repository);
    assert!(
        hashes.get(1..4).unwrap().contains(&between),
        "{between} is between A and E"
    );
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(says(&tree, &words::test(between.short(), 4)));
}

#[test]
fn bisect_mark__marking_the_last_commit_left_says_which_is_the_first_bad_one_with_the_two_buttons() {
    let (repository, hashes) = five_commits();
    let (c, d) = (at(&hashes, 'C'), at(&hashes, 'D'));
    bisect(&repository, &["start", at(&hashes, 'E').as_str(), c.as_str()]);
    assert_eq!(head(&repository), d, "git checks out D, between C and E");
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    give_until_ended(&driver, &mut state, marked("bisect.mark", &d, "bad"));
    let mut window = Window::new(ThemeName::Dark, WIDE);
    // What was said is drawn, and the store keeps when (`activity.said`).
    let (shown, tree) = window.draw(&state, &[]);
    super::pointer::apply(&mut state, shown);
    assert!(says(&tree, &words::first_bad(d.short(), "Add d")));
    assert!(has(&tree, Role::Button, words::SHOW_IN_TIMELINE));
    assert!(has(&tree, Role::Button, words::END_BISECT));
    assert_eq!(
        window.click(&state, Role::Button, words::SHOW_IN_TIMELINE),
        [Intent::Graph(GraphIntent::GoTo(d))]
    );
}

#[test]
fn bisect_answer__good_runs_git_bisect_good_and_the_count_of_the_banner_goes_down() {
    let (repository, hashes) = five_commits();
    bisect(
        &repository,
        &["start", at(&hashes, 'E').as_str(), at(&hashes, 'A').as_str()],
    );
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &words::test(head(&repository).short(), 4)));
    let driver = OperationsDriver::new();
    let good = window.click(&state, Role::Button, words::GOOD);
    give_until_ended(&driver, &mut state, good);
    assert_eq!(logged(&state), ["git bisect good"]);
    let remaining = state.shown_read().unwrap().state.bisect.as_ref().unwrap().remaining;
    assert!(remaining.is_some_and(|remaining| remaining < 4), "{remaining:?}");
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, &words::test(head(&repository).short(), 4)));
}

#[test]
fn bisect_answer__skip_runs_git_bisect_skip_and_the_commit_gets_the_mark_skipped() {
    let (repository, hashes) = five_commits();
    bisect(
        &repository,
        &["start", at(&hashes, 'E').as_str(), at(&hashes, 'A').as_str()],
    );
    let tested = head(&repository);
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let driver = OperationsDriver::new();
    let skip = window.click(&state, Role::Button, words::SKIP);
    give_until_ended(&driver, &mut state, skip);
    assert_eq!(logged(&state), ["git bisect skip"]);
    let bisect = state.shown_read().unwrap().state.bisect.clone().unwrap();
    assert_eq!(bisect.skipped, [tested]);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, words::SKIPPED_MARK));
}

#[test]
fn bisect_answer__the_answer_that_leaves_one_commit_says_the_first_bad_one_with_the_two_buttons() {
    let (repository, hashes) = five_commits();
    bisect(
        &repository,
        &["start", at(&hashes, 'E').as_str(), at(&hashes, 'C').as_str()],
    );
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let driver = OperationsDriver::new();
    let good = window.click(&state, Role::Button, words::GOOD);
    give_until_ended(&driver, &mut state, good);
    let e = at(&hashes, 'E');
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &words::first_bad(e.short(), "Add e")));
    assert!(has(&tree, Role::Button, words::SHOW_IN_TIMELINE));
    assert!(has(&tree, Role::Button, words::END_BISECT));
}

#[test]
fn bisect_banner__once_found_good_bad_and_skip_are_gone_and_show_selects_the_commit() {
    let (repository, hashes) = five_commits();
    let d = at(&hashes, 'D');
    bisect(&repository, &["start", d.as_str(), at(&hashes, 'C').as_str()]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, &words::found(d.short())));
    for gone in [words::GOOD, words::BAD, words::SKIP] {
        assert!(!has(&tree, Role::Button, gone), "{gone}");
    }
    assert_eq!(
        window.click(&state, Role::Button, words::SHOW),
        [Intent::Graph(GraphIntent::GoTo(d))]
    );
}

#[test]
fn bisect_banner__without_a_bisect_there_is_no_banner() {
    let (repository, _hashes) = five_commits();
    let state = showing(repository.path().to_path_buf());
    let (_, tree) = Window::new(ThemeName::Dark, WIDE).draw(&state, &[]);
    assert!(!has(&tree, Role::Group, words::BANNER));
    assert!(!has(&tree, Role::Button, words::END));
}

#[test]
fn bisect_end__started_on_main_end_runs_git_bisect_reset_and_main_is_checked_out_again() {
    let (repository, hashes) = five_commits();
    bisect(
        &repository,
        &["start", at(&hashes, 'E').as_str(), at(&hashes, 'A').as_str()],
    );
    let mut state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let driver = OperationsDriver::new();
    let end = window.click(&state, Role::Button, words::END);
    give_until_ended(&driver, &mut state, end);
    assert_eq!(logged(&state), ["git bisect reset"]);
    assert_eq!(repository.git(&["branch", "--show-current"]).trim(), "main");
    assert_eq!(state.shown_read().unwrap().state.bisect, None);
    let (_, tree) = window.draw(&state, &[]);
    assert!(!has(&tree, Role::Button, words::END));
}
