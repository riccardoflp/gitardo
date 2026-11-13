//! Rebasing and rewriting from the window: the banner of a rebase stopped to edit a commit, the interactive rebase page
//! opened, cancelled and started, and the commit a squash makes selected after it.

use app::store::{Message, State};
use model::CommitId;
use model::contract::{FeatureId, Target};
use model::rebase::TodoAction;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::feature::FeatureIntent;
use views::graph::Selected;
use views::page::Page;
use views::rebase::RebaseIntent;

use super::showing;
use super::support::{OperationsDriver, Window, has, says};

/// The size of the window.
const WIDE: (u32, u32) = (1100, 560);

/// A repository with four commits on `main`, `A` to `D`, `D` checked out.
fn four_commits() -> (Repository, Vec<CommitId>) {
    let mut repository = Repository::new();
    let commits = ["A", "B", "C", "D"]
        .iter()
        .map(|name| {
            let hash = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), name)]);
            CommitId::parse(hash.trim()).unwrap()
        })
        .collect();
    (repository, commits)
}

/// The subjects of `main`, newest first.
fn subjects(repository: &Repository) -> Vec<String> {
    repository
        .git(&["log", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Says yes to every plan shown until the operation ends, answering a question with its default.
fn to_the_end(driver: &OperationsDriver, state: &mut State) {
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureAsks(question) => {
                let answer = question.default.clone();
                driver.intent(state, Intent::Feature(FeatureIntent::Answer(answer)));
            }
            Message::FeatureEnded { .. } => return,
            _ => {}
        }
    }
}

#[test]
fn rebase_edit_stop__the_banner_has_continue_and_abort_and_no_skip_and_continue_runs_its_operation() {
    let (repository, _) = four_commits();
    let _ = repository.git(&[
        "-c",
        "sequence.editor=sed -i -e 1s/^pick/edit/",
        "rebase",
        "-i",
        "HEAD~2",
    ]);
    let state = showing(repository.path().to_path_buf());
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(
        &tree,
        "Stopped at \"Add C\" to edit it. Amend it or commit more, then Continue."
    ));
    assert!(has(&tree, Role::Button, "Continue"));
    assert!(has(&tree, Role::Button, "Abort"));
    assert!(!has(&tree, Role::Button, "Skip"));
    assert_eq!(
        window.click(&state, Role::Button, "Continue"),
        [Intent::Feature(FeatureIntent::Run(
            FeatureId("rebase.edit-stop"),
            Target::Head
        ))]
    );
}

#[test]
fn rebase_todo_page__opening_it_on_the_third_commit_under_head_lists_three_rows_head_first() {
    let (repository, commits) = four_commits();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let third = commits.get(1).cloned().unwrap();
    driver.intent(&mut state, Intent::Rebase(RebaseIntent::Open(third)));
    assert_eq!(state.page_shown(), Some(Page::Rebase));
    let rows: Vec<&str> = state
        .rebase_page
        .iter()
        .flat_map(|page| &page.rows)
        .map(|row| row.subject.as_str())
        .collect();
    assert_eq!(rows, ["Add D", "Add C", "Add B"]);
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "Interactive rebase of main: 3 commits"));
}

#[test]
fn rebase_todo_page__cancel_closes_the_page_and_runs_nothing() {
    let (repository, commits) = four_commits();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Rebase(RebaseIntent::Open(commits.get(1).cloned().unwrap())),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let intents = window.click(&state, Role::Button, "Cancel");
    assert_eq!(intents, [Intent::Rebase(RebaseIntent::Cancel)]);
    for intent in intents {
        driver.intent(&mut state, intent);
    }
    assert_eq!(state.page_shown(), None);
    assert_eq!(state.rebase_page, None);
    assert_eq!(state.operations.running, None, "nothing runs");
}

#[test]
fn rebase_todo_page__start_rebase_closes_the_page_and_calls_rebase_interactive_with_the_rows_oldest_first() {
    let (repository, commits) = four_commits();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Rebase(RebaseIntent::Open(commits.get(1).cloned().unwrap())),
    );
    driver.intent(&mut state, Intent::Rebase(RebaseIntent::SetAction(1, TodoAction::Drop)));
    driver.intent(&mut state, Intent::Rebase(RebaseIntent::Move { from: 2, to: 0 }));
    driver.intent(&mut state, Intent::Rebase(RebaseIntent::Start));
    assert_eq!(state.page_shown(), None);
    to_the_end(&driver, &mut state);
    assert_eq!(subjects(&repository), ["Add B", "Add D", "Add A"]);
}

#[test]
fn squash_commits__the_commit_they_become_is_selected_after_it() {
    let (repository, commits) = four_commits();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let selected = commits.get(1..3).unwrap_or_default().to_vec();
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::Run(
            FeatureId("squash.commits"),
            Target::Commits(selected),
        )),
    );
    to_the_end(&driver, &mut state);
    assert_eq!(subjects(&repository), ["Add D", "Add B", "Add A"]);
    let squashed = repository.git(&["rev-parse", "HEAD~1"]);
    let squashed = CommitId::parse(squashed.trim()).unwrap();
    assert_eq!(state.timeline.selected, Some(Selected::Commit(squashed)));
}
