//! The operations that bring commits onto what is checked out, run from the window: a squash merge and its Undo, the
//! message git prepared for the draft, several commits picked at once and taken back by one Undo, and the bar of a
//! selection of several.

use std::time::Duration;

use app::store::{Message, State};
use model::CommitId;
use model::contract::{Answer, Answers, FeatureId, Target};
use model::repo::Change;
use platform::accesskit::Role;
use render::ThemeName;
use testkit::Repository;
use views::Intent;
use views::changes::ChangesIntent;
use views::feature::FeatureIntent;
use views::graph::{GraphIntent, Selected};

use super::showing;
use super::support::{OperationsDriver, Window, has, says};

/// The size of the window of the tests.
const WIDE: (u32, u32) = (1000, 520);

/// `main` and `feature` apart from `Add the readme`: `main` with `Add m`, `feature` with three commits, each adding a
/// file of its own; `main` checked out. The three commits of `feature`, oldest first.
fn three_on_feature() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let _base = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    repository.branch("feature");
    let _main = repository.commit("Add m", &[("m.txt", "m\n")]);
    repository.checkout("feature");
    let picked = ["a", "b", "c"]
        .iter()
        .map(|name| repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "feature\n")]))
        .collect();
    repository.checkout("main");
    (repository, picked)
}

/// `main` and `feature` that both change `shared.txt` since `Add the readme`, `main` checked out.
fn both_change_a_file() -> Repository {
    let mut repository = Repository::new();
    let _base = repository.commit(
        "Add the readme",
        &[("README.md", "Gitardo\n"), ("shared.txt", "base\n")],
    );
    repository.branch("feature");
    let _main = repository.commit("Change shared on main", &[("shared.txt", "main\n")]);
    repository.checkout("feature");
    let _feature = repository.commit("Change shared on feature", &[("shared.txt", "feature\n")]);
    repository.checkout("main");
    repository
}

/// The hash of a commit as git prints it.
fn commit_id(hash: &str) -> CommitId {
    CommitId::parse(hash.trim()).unwrap()
}

/// Tells the store a branch and the index changed, as the watcher does after a commit in a terminal, and lets the read
/// it asks for run.
fn read_again(state: &mut State) {
    let changes = vec![
        Change::GitFolder("refs/heads/main".into()),
        Change::GitFolder("index".into()),
    ];
    let _ = super::support::run(
        state,
        Message::Changed {
            changes,
            at: Duration::from_secs(10),
        },
    );
    let _ = super::support::run(
        state,
        Message::Wake {
            at: Duration::from_secs(20),
        },
    );
}

/// Runs an operation of the catalog as a click on its entry does.
fn run(driver: &OperationsDriver, state: &mut State, feature: &'static str, target: Target) {
    driver.intent(state, Intent::Feature(FeatureIntent::Run(FeatureId(feature), target)));
}

/// Merges `feature` into `main` with `git merge --squash`, as choosing **Merge (squash)** in the pick does.
fn squash_feature(driver: &OperationsDriver, state: &mut State) -> Vec<String> {
    let way = Answers::new().with(features::merge::WAY, Answer::Choice(String::from("squash")));
    let target = Target::Branch(String::from("feature"));
    driver.intent(
        state,
        Intent::Feature(FeatureIntent::RunWith(FeatureId("merge.choose-way"), target, way)),
    );
    confirm_until_ended(driver, state)
}

/// Waits until the operation ends, confirming the plan it shows first, and gives the lines of the Command log it
/// added.
fn confirm_until_ended(driver: &OperationsDriver, state: &mut State) -> Vec<String> {
    loop {
        match driver.next(state) {
            Message::FeatureConfirms(_) => driver.intent(state, Intent::Feature(FeatureIntent::Confirm(true))),
            Message::FeatureEnded { log, .. } => return log.into_iter().map(|entry| entry.line).collect(),
            _ => {}
        }
    }
}

#[test]
fn merge_undo_squash__right_after_a_squash_merge_undo_runs_git_reset_merge_and_nothing_is_staged() {
    let (repository, _picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _merged = squash_feature(&driver, &mut state);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]).lines().count(), 3);
    run(&driver, &mut state, "undo.last", Target::Head);
    let lines = confirm_until_ended(&driver, &mut state);
    assert!(lines.contains(&String::from("git reset --merge")), "{lines:?}");
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "");
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn undo_last__after_a_squash_merge_with_nothing_committed_since_undo_runs_git_reset_merge_and_the_index_is_clean() {
    let (repository, _picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _merged = squash_feature(&driver, &mut state);
    run(&driver, &mut state, "undo.last", Target::Head);
    let lines = confirm_until_ended(&driver, &mut state);
    assert_eq!(lines, ["git reset --merge"]);
    assert_eq!(repository.git(&["diff", "--cached", "--stat"]), "");
}

#[test]
fn merge_undo_squash__after_a_squash_merge_that_stopped_on_conflicts_undo_leaves_no_file_in_conflict() {
    let repository = both_change_a_file();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _stopped = squash_feature(&driver, &mut state);
    assert_eq!(
        repository.git(&["ls-files", "-u", "--", "shared.txt"]).lines().count(),
        3
    );
    run(&driver, &mut state, "undo.last", Target::Head);
    let lines = confirm_until_ended(&driver, &mut state);
    assert!(lines.contains(&String::from("git reset --merge")), "{lines:?}");
    assert_eq!(repository.git(&["ls-files", "-u"]), "");
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}

#[test]
fn merge_undo_squash__after_a_squash_merge_and_a_commit_undo_takes_back_the_commit_by_the_reflog() {
    let (repository, _picked) = three_on_feature();
    let before = repository.git(&["rev-parse", "HEAD"]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _merged = squash_feature(&driver, &mut state);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Bring the feature"]);
    read_again(&mut state);
    run(&driver, &mut state, "undo.last", Target::Head);
    let lines = confirm_until_ended(&driver, &mut state);
    assert!(!lines.iter().any(|line| line.contains("reset --merge")), "{lines:?}");
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before);
}

#[test]
fn merge_choose_way__after_a_squash_the_empty_message_of_the_draft_holds_what_git_prepared() {
    let (repository, _picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let _merged = squash_feature(&driver, &mut state);
    let prepared = std::fs::read_to_string(repository.path().join(".git").join("SQUASH_MSG")).unwrap();
    assert!(prepared.starts_with("Squashed commit of the following:"), "{prepared}");
    let kept = state.persisted.commit_messages.first().map(|kept| kept.message.clone());
    assert_eq!(kept.as_deref(), Some(prepared.trim_end()));
}

#[test]
fn merge_choose_way__after_a_squash_a_draft_message_with_text_keeps_it() {
    let (repository, _picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    driver.intent(
        &mut state,
        Intent::Changes(ChangesIntent::Type(String::from("Bring the feature"))),
    );
    let _merged = squash_feature(&driver, &mut state);
    let kept = state.persisted.commit_messages.first().map(|kept| kept.message.clone());
    assert_eq!(kept.as_deref(), Some("Bring the feature"));
}

#[test]
fn selection_cherry_pick__after_three_commits_picked_one_undo_resets_keep_to_where_head_was() {
    let (repository, picked) = three_on_feature();
    let before = repository.git(&["rev-parse", "HEAD"]);
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let newest_first = picked.iter().rev().map(|hash| commit_id(hash)).collect();
    run(
        &driver,
        &mut state,
        "selection.cherry-pick",
        Target::Commits(newest_first),
    );
    let lines = confirm_until_ended(&driver, &mut state);
    let oldest_first: Vec<&str> = picked.iter().map(|hash| hash.trim()).collect();
    assert_eq!(lines, [format!("git cherry-pick {}", oldest_first.join(" "))]);
    assert_eq!(repository.git(&["rev-list", "--count", "main"]).trim(), "5");
    run(&driver, &mut state, "undo.last", Target::Head);
    let undone = confirm_until_ended(&driver, &mut state);
    assert_eq!(undone, [format!("git reset --keep {}", before.trim())]);
    assert_eq!(repository.git(&["rev-parse", "HEAD"]), before);
}

#[test]
fn selection_bar__clear_selection_leaves_no_commit_selected_together_and_the_bar_goes_away() {
    let (repository, picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let head = commit_id(&repository.git(&["rev-parse", "HEAD"]));
    let readme = commit_id(&repository.git(&["rev-parse", "HEAD~1"]));
    driver.intent(
        &mut state,
        Intent::Graph(GraphIntent::Select(Selected::Commit(head.clone()))),
    );
    driver.intent(
        &mut state,
        Intent::Graph(GraphIntent::SelectSeveral {
            commits: vec![head, readme.clone()],
            shown: readme,
        }),
    );
    let mut window = Window::new(ThemeName::Dark, WIDE);
    let (_, tree) = window.draw(&state, &[]);
    assert!(says(&tree, "2 commits selected"));
    assert!(has(&tree, Role::Button, "Squash into one"));
    window.assert_snapshot("selection-bar-dark");
    let cleared = window.click(&state, Role::Button, "Clear selection");
    assert_eq!(cleared, [Intent::Graph(GraphIntent::ClearSelection)]);
    for intent in cleared {
        driver.intent(&mut state, intent);
    }
    let (_, tree) = window.draw(&state, &[]);
    assert!(!says(&tree, "2 commits selected"));
    assert_eq!(picked.len(), 3);
}

#[test]
fn selection_menu__copy_hashes_puts_one_full_hash_per_line_in_the_clipboard_in_the_order_of_the_timeline() {
    let (repository, _picked) = three_on_feature();
    let mut state = showing(repository.path().to_path_buf());
    let hashes: Vec<CommitId> = repository
        .git(&["rev-list", "--all", "--max-count=3", "--topo-order"])
        .lines()
        .map(commit_id)
        .collect();
    let effects = super::support::run(
        &mut state,
        Message::Intent(Intent::Graph(GraphIntent::CopyHashes(hashes.clone()))),
    );
    let copied = effects.iter().find_map(|effect| match effect {
        app::store::Effect::CopyText(text) => Some(text.clone()),
        _ => None,
    });
    let copied = copied.unwrap_or_default();
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(
        lines
            .iter()
            .all(|line| line.len() == 40 && line.chars().all(|character| character.is_ascii_hexdigit()))
    );
    assert_eq!(lines, hashes.iter().map(CommitId::as_str).collect::<Vec<_>>());
}

#[test]
fn message_edit__a_new_message_for_the_third_commit_under_head_selects_that_commit_made_again() {
    let mut repository = Repository::new();
    for name in ["a", "b", "c", "d"] {
        let _ = repository.commit(&format!("Add {name}"), &[(&format!("{name}.txt"), "text\n")]);
    }
    let third = commit_id(&repository.git(&["rev-parse", "HEAD~2"]));
    let mut state = showing(repository.path().to_path_buf());
    let driver = OperationsDriver::new();
    let message = Answers::new().with(features::message::MESSAGE, Answer::Text(String::from("Add the b file")));
    driver.intent(
        &mut state,
        Intent::Feature(FeatureIntent::RunWith(
            FeatureId("message.edit"),
            Target::Commit(third),
            message,
        )),
    );
    let _ran = confirm_until_ended(&driver, &mut state);
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s", "HEAD~2"]).trim(),
        "Add the b file"
    );
    let made_again = commit_id(&repository.git(&["rev-parse", "HEAD~2"]));
    assert_eq!(state.timeline.selected, Some(Selected::Commit(made_again)));
}
