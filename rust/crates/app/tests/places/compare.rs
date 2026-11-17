//! The comparison page: its sides, what it says of them, its commits and files, swapped, in another mode, narrowed to
//! a folder, and read again when the working tree changes.

use std::time::Duration;

use app::store::{Effect, Message, State, reduce};
use app::work::jobs;
use git::CancelToken;
use model::compare::CompareMode;
use model::contract::{Answer, AnswerKey, Answers, FeatureId, QuestionKind, Target};
use model::repo::Change;
use platform::accesskit::{Role, TreeUpdate};
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::compare::{CompareIntent, ComparePageId};
use views::diff::DiffVersion;
use views::feature::FeatureIntent;
use views::page::Page;

use super::histories::{imported, line};
use super::showing;
use super::support::{OperationsDriver, Window, has, run, says, says_in};

/// `main` at `Add a` then `Add d`, and `feature` from `Add a` with `Add b` (in `src`) and `Add c`; `main` checked out.
pub fn repository() -> Repository {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _b = repository.commit("Add b", &[("src/b.txt", "b\n")]);
    let _c = repository.commit("Add c", &[("c.txt", "c\n")]);
    repository.checkout("main");
    let _d = repository.commit("Add d", &[("d.txt", "d\n")]);
    repository
}

/// Waits until the operation running ends, confirming its plan.
pub fn until_ended(driver: &OperationsDriver, state: &mut State) {
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

/// A window showing the comparison page of two sides chosen in **Compare branches, tags or commits…**, by their
/// identifiers: `ref:<name>`, `working-tree`, or `revision` with the revision typed.
pub fn compared(repository: &Repository, (first, second): (&str, &str)) -> (State, OperationsDriver) {
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let mut answers = Answers::new();
    for (key, typed, side) in [
        ("first", "first-revision", first),
        ("second", "second-revision", second),
    ] {
        match side.strip_prefix("revision:") {
            Some(revision) => {
                answers.insert(AnswerKey(key), Answer::Choice(String::from("revision")));
                answers.insert(AnswerKey(typed), Answer::Text(revision.to_owned()));
            }
            None => answers.insert(AnswerKey(key), Answer::Choice(side.to_owned())),
        }
    }
    let choose = FeatureIntent::RunWith(FeatureId("compare.choose"), Target::Head, answers);
    driver.intent(&mut state, Intent::Feature(choose));
    until_ended(&driver, &mut state);
    (state, driver)
}

/// The page shown, which must be a comparison page.
pub fn page_id(state: &State) -> ComparePageId {
    match state.page_shown() {
        Some(Page::Compare(id)) => id,
        shown => panic!("the comparison page is not shown: {shown:?}"),
    }
}

/// What the window shows.
fn drawn(state: &State) -> TreeUpdate {
    Window::new(ThemeName::Dark, (1200, 900)).draw(state, &[]).1
}

/// Reduces what a view returned, with what it starts.
fn apply(driver: &OperationsDriver, state: &mut State, intents: Vec<Intent>) {
    for intent in intents {
        driver.intent(state, intent);
    }
}

/// The paths of the files a comparison page lists.
pub fn files(state: &State) -> Vec<String> {
    let page = state
        .compares
        .pages
        .iter()
        .find(|page| page.id == page_id(state))
        .unwrap();
    match &page.read {
        app::store::compare::PageRead::Read(view) => view.files.iter().map(|file| file.path.clone()).collect(),
        read => panic!("not read: {read:?}"),
    }
}

/// Narrows the page shown to what is typed, as `Enter` in its box does.
fn narrow(driver: &OperationsDriver, state: &mut State, typed: &str) {
    let id = page_id(state);
    driver.intent(state, Intent::Compare(CompareIntent::TypePath(id, typed.to_owned())));
    driver.intent(state, Intent::Compare(CompareIntent::Narrow(id)));
}

#[test]
fn compare_page__main_and_feature_show_both_names_their_hashes_the_distance_commits_and_files() {
    let repository = repository();
    let (state, _driver) = compared(&repository, ("ref:main", "ref:feature"));
    let tree = drawn(&state);
    let short = |name: &str| repository.git(&["rev-parse", "--short=7", name]).trim().to_owned();
    for name in ["main", "feature"] {
        assert!(has(&tree, Role::Button, name), "{name}");
        assert!(has(&tree, Role::Button, &short(name)), "{name}");
    }
    assert!(says_in(&tree, "is 2 commits ahead of"));
    assert!(says_in(&tree, "Add b \u{b7} Test Author"));
    assert!(says_in(&tree, "src/b.txt"));
    assert!(says(&tree, "2 changed files"));
}

#[test]
fn compare_page__a_click_on_the_short_hash_of_a_side_opens_the_page_of_that_commit() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let short = repository.git(&["rev-parse", "--short=7", "feature"]).trim().to_owned();
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, &short);
    apply(&driver, &mut state, intents);
    assert!(
        matches!(state.page_shown(), Some(Page::Commit(_))),
        "{:?}",
        state.page_shown()
    );
}

#[test]
fn compare_page__editing_a_file_of_the_working_tree_updates_a_comparison_with_it_without_any_action() {
    let repository = repository();
    let (mut state, _driver) = compared(&repository, ("ref:main", "working-tree"));
    assert_eq!(files(&state), Vec::<String>::new());
    std::fs::write(repository.path().join("a.txt"), "changed\n").unwrap();
    let changes = vec![Change::WorkingTree("a.txt".into())];
    let _ = run(
        &mut state,
        Message::Changed {
            changes,
            at: Duration::from_secs(10),
        },
    );
    let _ = run(
        &mut state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    );
    assert_eq!(files(&state), ["a.txt"]);
    assert!(says(&drawn(&state), "1 changed file"));
}

#[test]
fn compare_page__open_in_the_diff_page_opens_all_the_files_listed_in_one_diff_page() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "Open in the Diff page");
    apply(&driver, &mut state, intents);
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    let page = state.diff_pages.pages.last().unwrap();
    let opened: Vec<&str> = page.target.files.iter().map(|file| file.right.as_str()).collect();
    assert_eq!(opened, ["c.txt", "src/b.txt"]);
}

#[test]
fn compare_page__open_on_github_is_not_there_for_a_repository_not_on_github_nor_with_the_working_tree() {
    let repository = repository();
    let (state, _driver) = compared(&repository, ("ref:main", "ref:feature"));
    assert!(!has(&drawn(&state), Role::Button, "Open on GitHub"));
    let (state, _driver) = compared(&repository, ("ref:feature", "working-tree"));
    assert!(!has(&drawn(&state), Role::Button, "Open on GitHub"));
}

#[test]
fn compare_page__a_click_on_a_file_opens_its_diff_page_between_the_two_sides_as_the_mode_says() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "src/b.txt");
    apply(&driver, &mut state, intents);
    assert!(
        matches!(state.page_shown(), Some(Page::Diff(_))),
        "{:?}",
        state.page_shown()
    );
    let page = state.diff_pages.pages.last().unwrap();
    let base = model::CommitId::parse(repository.git(&["merge-base", "main", "feature"]).trim()).unwrap();
    assert_eq!(page.target.left, DiffVersion::commit(base));
    let feature = model::CommitId::parse(repository.git(&["rev-parse", "feature"]).trim()).unwrap();
    assert_eq!(
        page.target.right,
        DiffVersion::Commit {
            commit: feature,
            name: String::from("feature")
        }
    );
}

#[test]
fn compare_page__closing_the_page_while_it_is_compared_drops_the_answer_without_error() {
    let repository = repository();
    let (mut state, _driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    let effects = reduce(&mut state, Message::Intent(Intent::Compare(CompareIntent::Swap(id))));
    let job = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Start(job) => Some(job.clone()),
            _ => None,
        })
        .unwrap();
    let _closed = reduce(&mut state, Message::Intent(Intent::ClosePage));
    let answer = jobs::messages(&job, &CancelToken::new()).into_iter().next().unwrap();
    let _dropped = reduce(&mut state, answer);
    assert_eq!(state.compares.pages, []);
    assert_eq!(state.page_shown(), None);
}

#[test]
fn compare_swap__main_feature_becomes_feature_main_with_the_counts_exchanged_and_the_files_the_other_way() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "Compare in the other direction");
    apply(&driver, &mut state, intents);
    let tree = drawn(&state);
    assert!(says(&tree, "feature \u{2194} main"));
    assert!(says_in(&tree, "is 1 commit ahead of"));
    assert!(says_in(&tree, "and 2 commits behind"));
    assert_eq!(files(&state), ["d.txt"]);
}

#[test]
fn compare_swap__the_mode_and_the_path_are_kept() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(
        &mut state,
        Intent::Compare(CompareIntent::SetMode(id, CompareMode::Everything)),
    );
    narrow(&driver, &mut state, "src");
    driver.intent(&mut state, Intent::Compare(CompareIntent::Swap(id)));
    let target = &state.compares.pages.first().unwrap().target;
    assert_eq!(
        (target.first.name.as_str(), target.second.name.as_str()),
        ("feature", "main")
    );
    assert_eq!(target.mode, CompareMode::Everything);
    assert_eq!(target.path.as_deref(), Some("src"));
}

#[test]
fn compare_swap__the_swapped_comparison_is_the_latest_opened_before_in_place_of_the_other_way() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(&mut state, Intent::Compare(CompareIntent::Swap(id)));
    let choose = FeatureIntent::Run(FeatureId("compare.choose"), Target::Head);
    driver.intent(&mut state, Intent::Feature(choose));
    let asked = loop {
        if let Message::FeatureAsks(question) = driver.next(&mut state) {
            break question;
        }
    };
    let QuestionKind::Pick { choices } = &asked.kind else {
        panic!("{asked:?}");
    };
    let labels: Vec<&str> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels.first(), Some(&"feature \u{2194} main"), "{labels:?}");
    assert!(!labels.contains(&"main \u{2194} feature"), "{labels:?}");
}

#[test]
fn compare_set_mode__since_they_diverged_lists_only_the_files_the_second_changed_since_the_merge_base() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let id = page_id(&state);
    driver.intent(
        &mut state,
        Intent::Compare(CompareIntent::SetMode(id, CompareMode::Everything)),
    );
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "Since they diverged");
    apply(&driver, &mut state, intents);
    assert_eq!(files(&state), ["c.txt", "src/b.txt"]);
}

#[test]
fn compare_set_mode__everything_that_differs_also_lists_the_files_the_first_changed_since_then() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "Everything that differs");
    apply(&driver, &mut state, intents);
    assert_eq!(files(&state), ["c.txt", "d.txt", "src/b.txt"]);
}

#[test]
fn compare_narrow_to_path__src_and_enter_lists_only_its_files_and_commits_and_the_title_ends_with_it() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    narrow(&driver, &mut state, "src");
    assert_eq!(files(&state), ["src/b.txt"]);
    let tree = drawn(&state);
    assert!(says_in(&tree, "Add b \u{b7} Test Author"));
    assert!(!says_in(&tree, "Add c \u{b7} Test Author"));
    assert!(says(&tree, "main \u{2194} feature \u{b7} src"));
}

#[test]
fn compare_narrow_to_path__the_count_of_a_side_then_reads_n_of_m_change_the_path() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    narrow(&driver, &mut state, "src");
    assert!(says(&drawn(&state), "Only in feature \u{b7} 1 of 2 change src"));
}

#[test]
fn compare_narrow_to_path__dot_slash_src_slash_typed_narrows_to_src() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    narrow(&driver, &mut state, "./src/");
    assert_eq!(
        state.compares.pages.first().unwrap().target.path.as_deref(),
        Some("src")
    );
}

#[test]
fn compare_narrow_to_path__the_cross_shows_every_file_and_commit_again() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    narrow(&driver, &mut state, "src");
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, "Show the changes of every file");
    apply(&driver, &mut state, intents);
    assert_eq!(files(&state), ["c.txt", "src/b.txt"]);
    let tree = drawn(&state);
    assert!(says_in(&tree, "Add c \u{b7} Test Author"));
    assert!(says(&tree, "main \u{2194} feature"));
}

#[test]
fn compare_commits__a_commit_only_in_the_second_side_is_under_only_in_it_with_its_author_and_date() {
    let repository = repository();
    let (state, _driver) = compared(&repository, ("ref:main", "ref:feature"));
    let tree = drawn(&state);
    assert!(says(&tree, "Only in feature \u{b7} 2"));
    let short = repository.git(&["rev-parse", "--short=7", "feature"]).trim().to_owned();
    let row = format!("{short}  Add c \u{b7} Test Author \u{b7} ");
    assert!(says_in(&tree, &row), "{row}");
}

#[test]
fn compare_commits__a_commit_cherry_picked_from_the_first_side_says_also_in_it_and_is_counted() {
    let mut repository = repository();
    repository.checkout("feature");
    let _picked = repository.git(&["cherry-pick", "main"]);
    repository.checkout("main");
    let _again = repository.commit("Add e", &[("e.txt", "e\n")]);
    let (state, _driver) = compared(&repository, ("ref:main", "ref:feature"));
    let tree = drawn(&state);
    assert!(says(&tree, "Only in feature \u{b7} 3, 1 also in main"));
    assert!(says_in(&tree, "Add d \u{b7} Test Author"));
    assert!(says_in(&tree, "also in main"));
}

#[test]
fn compare_commits__with_2500_commits_the_heading_says_too_many_2000_are_read_and_the_newest_are_listed() {
    let repository = imported(&line(2501));
    let (state, _driver) = compared(&repository, ("revision:HEAD~2500", "ref:main"));
    let page = state.compares.pages.first().unwrap();
    let app::store::compare::PageRead::Read(view) = &page.read else {
        panic!("not read: {:?}", page.read);
    };
    assert_eq!(view.commits.len(), 2000);
    let tree = drawn(&state);
    assert!(says(&tree, "Too many to look for the same changes on the other side"));
    assert!(says(&tree, "The newest 2000 are listed."));
}

#[test]
fn compare_commits__a_click_on_a_commit_opens_its_page() {
    let repository = repository();
    let (mut state, driver) = compared(&repository, ("ref:main", "ref:feature"));
    let short = repository.git(&["rev-parse", "--short=7", "feature"]).trim().to_owned();
    let tree = drawn(&state);
    let row = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label())
        .find(|label| label.starts_with(&format!("{short}  Add c")))
        .unwrap()
        .to_owned();
    let mut window = Window::new(ThemeName::Dark, (1200, 900));
    let intents = window.click(&state, Role::Button, &row);
    apply(&driver, &mut state, intents);
    assert!(
        matches!(state.page_shown(), Some(Page::Commit(_))),
        "{:?}",
        state.page_shown()
    );
}
