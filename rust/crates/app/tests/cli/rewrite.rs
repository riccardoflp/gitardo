use std::sync::Arc;

use engine::{Asker, CommandLog, Engine, Finished, FixedClock, LogLevel};
use git::Git;
use git::read::repo::{StateRequest, read_state};
use model::Timestamp;
use model::contract::{Answer, Context, FeatureId, FeatureSettings, Plan, Question, Target};
use testkit::Repository;

use crate::{cli, cli_done, revision, write};

/// `A→B→C→D→E`, `E` checked out: the full hashes, oldest first.
fn five() -> (Repository, Vec<String>) {
    let mut repository = Repository::new();
    let hashes = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|name| {
            let file = format!("{name}.txt");
            repository.commit(&format!("Add {name}"), &[(file.as_str(), name)])
        })
        .collect();
    (repository, hashes)
}

/// The hash of the commit at a place of the line, `A` being 0.
fn at(hashes: &[String], place: usize) -> &str {
    hashes.get(place).map_or("", String::as_str)
}

/// The subjects of the commits of `HEAD`, newest first.
fn subjects(repository: &Repository) -> Vec<String> {
    repository
        .git(&["log", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// What `git status --porcelain` says.
fn status(repository: &Repository) -> String {
    repository.git(&["status", "--porcelain"])
}

#[test]
fn squash_commits__b_c_and_d_become_one_commit_selected_under_e() {
    let (repository, hashes) = five();
    let target = format!("commits:{},{},{}", at(&hashes, 3), at(&hashes, 1), at(&hashes, 2));
    let ended = cli_done(
        &repository,
        &["--yes", "--target", &target, "squash.commits", "subject=Add b, c and d"],
    );
    assert!(ended.out.contains("Select HEAD~1"), "{}", ended.out);
    assert_eq!(subjects(&repository), ["Add e", "Add b, c and d", "Add a"]);
    assert_eq!(revision(&repository, "HEAD~2"), at(&hashes, 0));
    let files = repository.git(&["show", "--name-only", "--format=", "HEAD~1"]);
    assert_eq!(files, "b.txt\nc.txt\nd.txt\n");
    let message = repository.git(&["log", "-1", "--format=%B", "HEAD~1"]);
    assert_eq!(message, "Add b, c and d\n\nAdd b\n\nAdd c\n\nAdd d\n\n");
}

#[test]
fn squash_commits__closing_the_box_runs_nothing() {
    let (repository, hashes) = five();
    let target = format!("commits:{},{}", at(&hashes, 1), at(&hashes, 2));
    let ended = cli(&repository, &["--yes", "--target", &target, "squash.commits"]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.errors.contains("Squash 2 commits"), "{}", ended.errors);
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 4));
}

#[test]
fn squash_commits__a_file_modified_is_modified_again_after_it() {
    let (repository, hashes) = five();
    write(&repository, "a.txt", "a, changed");
    let target = format!("commits:{},{}", at(&hashes, 2), at(&hashes, 3));
    let ended = cli_done(
        &repository,
        &["--yes", "--target", &target, "squash.commits", "subject=Add c and d"],
    );
    assert!(ended.out.contains("set aside and put back"), "{}", ended.out);
    assert_eq!(subjects(&repository), ["Add e", "Add c and d", "Add b", "Add a"]);
    assert_eq!(status(&repository), " M a.txt\n");
}

#[test]
fn squash_commits__one_undo_puts_the_branch_back_on_e() {
    let (repository, hashes) = five();
    let target = format!("commits:{},{}", at(&hashes, 1), at(&hashes, 2));
    let _squashed = cli_done(
        &repository,
        &["--yes", "--target", &target, "squash.commits", "subject=Squashed"],
    );
    let _undone = cli_done(&repository, &["--yes", "undo.last"]);
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 4));
}

#[test]
fn squash_into_previous__on_head_makes_one_commit_of_the_two() {
    let (repository, hashes) = five();
    let target = format!("commit:{}", at(&hashes, 4));
    let _ended = cli_done(
        &repository,
        &[
            "--yes",
            "--target",
            &target,
            "squash.into-previous",
            "subject=Add d and e",
        ],
    );
    assert_eq!(subjects(&repository), ["Add d and e", "Add c", "Add b", "Add a"]);
}

#[test]
fn split_commit__the_last_commit_goes_back_to_the_working_tree_unstaged() {
    let (repository, hashes) = five();
    let target = format!("commit:{}", at(&hashes, 4));
    let ended = cli_done(&repository, &["--yes", "--target", &target, "split.commit"]);
    assert!(ended.out.contains("> git reset --mixed HEAD~"), "{}", ended.out);
    assert!(ended.out.contains("Commit message: Add e"), "{}", ended.out);
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 3));
    assert_eq!(status(&repository), "?? e.txt\n");
}

/// Answers nothing, and runs every plan.
struct RunsAll;

impl Asker for RunsAll {
    fn ask(&mut self, _question: &Question) -> Option<Answer> {
        None
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        true
    }
}

/// Runs an operation through an engine kept from one to the next, as the window does, so that its journal holds.
fn run(engine: &mut Engine, repository: &Repository, (feature, target): (&'static str, Target)) -> Finished {
    let git = Git::new(repository.path());
    let request = StateRequest {
        folder: repository.path().to_path_buf(),
    };
    let state = read_state(&git, &request).unwrap_or_else(|error| panic!("the state is read: {error}"));
    let context = Context {
        repository: Arc::new(state),
        target,
        settings: FeatureSettings::default(),
        now: Timestamp(0),
    };
    let Some((identifier, operation)) = features::operation(feature) else {
        panic!("{feature} is built")
    };
    assert_eq!(identifier, FeatureId(feature));
    engine
        .run_feature(identifier, operation, &context, &mut RunsAll)
        .unwrap_or_else(|error| panic!("{feature} ran: {error}"))
}

#[test]
fn split_commit__undo_right_after_puts_head_back_with_a_file_it_added_tracked_again() {
    let (repository, hashes) = five();
    let mut engine = Engine::new(
        Git::new(repository.path()),
        CommandLog::new(LogLevel::Info),
        Box::new(FixedClock(Timestamp(0))),
    );
    let split = Target::Commit(model::CommitId::parse(at(&hashes, 4)).unwrap_or_else(|| panic!("a hash")));
    assert!(matches!(
        run(&mut engine, &repository, ("split.commit", split)),
        Finished::Ran { .. }
    ));
    assert_eq!(status(&repository), "?? e.txt\n");
    assert!(matches!(
        run(&mut engine, &repository, ("undo.last", Target::Head)),
        Finished::Ran { .. }
    ));
    assert_eq!(revision(&repository, "HEAD"), at(&hashes, 4));
    assert_eq!(status(&repository), "");
}

/// Splits `D`, the commit under `E`: the rebase stops with its changes unstaged.
fn split_d(repository: &Repository, hashes: &[String]) -> crate::Ended {
    let target = format!("commit:{}", at(hashes, 3));
    cli_done(repository, &["--yes", "--target", &target, "split.commit"])
}

#[test]
fn split_commit__the_second_under_head_stops_the_rebase_with_its_changes_unstaged() {
    let (repository, hashes) = five();
    let ended = split_d(&repository, &hashes);
    assert!(ended.out.contains("Commit message: Add d"), "{}", ended.out);
    let split = std::fs::read_to_string(repository.path().join(".git/rebase-merge/gitardo-split"))
        .unwrap_or_else(|error| panic!("gitardo-split is written: {error}"));
    assert_eq!(split, format!("{}\n", at(&hashes, 3)));
    assert_eq!(status(&repository), "?? d.txt\n");
    let state = cli_done(&repository, &["state"]);
    assert!(state.out.contains("Stopped     to split"), "{}", state.out);
}

#[test]
fn split_continue__with_its_changes_committed_in_two_commits_continues_on_top() {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    let both = repository.commit("Add b and c", &[("b.txt", "b"), ("c.txt", "c")]);
    let _d = repository.commit("Add d", &[("d.txt", "d")]);
    let target = format!("commit:{both}");
    let _split = cli_done(&repository, &["--yes", "--target", &target, "split.commit"]);
    let _b = repository.commit("Add b", &[("b.txt", "b")]);
    let _c = repository.commit("Add c", &[("c.txt", "c")]);
    let ended = cli_done(&repository, &["split.continue"]);
    assert!(
        ended.out.contains("> git -c core.editor=true rebase --continue"),
        "{}",
        ended.out
    );
    assert_eq!(subjects(&repository), ["Add d", "Add c", "Add b", "Add a"]);
    assert!(!repository.path().join(".git/gitardo-rebase").exists());
}

#[test]
fn split_continue__commits_what_is_left_with_the_message_of_the_commit() {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    let both = repository.commit("Add b and c", &[("b.txt", "b"), ("c.txt", "c")]);
    let _d = repository.commit("Add d", &[("d.txt", "d")]);
    let target = format!("commit:{both}");
    let _split = cli_done(&repository, &["--yes", "--target", &target, "split.commit"]);
    let _b = repository.commit("Add b", &[("b.txt", "b")]);
    let asked = cli(&repository, &["split.continue"]);
    assert!(
        asked.errors.contains("1 file of it is still changed"),
        "{}
{}",
        asked.out,
        asked.errors
    );
    let _ended = cli_done(&repository, &["split.continue", "left=commit"]);
    assert_eq!(subjects(&repository), ["Add d", "Add b and c", "Add b", "Add a"]);
    assert_eq!(status(&repository), "");
}

#[test]
fn split_continue__a_file_not_part_of_it_modified_is_refused_by_its_name() {
    let (repository, hashes) = five();
    let _split = split_d(&repository, &hashes);
    let _d = repository.git(&["add", "d.txt"]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Add d again"]);
    write(&repository, "a.txt", "a, changed");
    let ended = cli(&repository, &["split.continue"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("a.txt has changes that are not committed"),
        "{}",
        ended.errors
    );
}

#[test]
fn split_continue__an_untracked_file_not_part_of_it_does_not_keep_it_from_continuing() {
    let (repository, hashes) = five();
    let _split = split_d(&repository, &hashes);
    let _d = repository.git(&["add", "d.txt"]);
    let _committed = repository.git(&["commit", "--quiet", "-m", "Add d again"]);
    write(&repository, "notes.txt", "notes");
    let _ended = cli_done(&repository, &["split.continue"]);
    assert_eq!(
        subjects(&repository),
        ["Add e", "Add d again", "Add c", "Add b", "Add a"]
    );
}

#[test]
fn fixup_commit__adds_the_staged_change_to_the_commit_and_leaves_the_rest_modified() {
    let (repository, hashes) = five();
    write(&repository, "a.txt", "a, fixed");
    let _staged = repository.git(&["add", "a.txt"]);
    write(&repository, "b.txt", "b, changed");
    let target = format!("commit:{}", at(&hashes, 2));
    let ended = cli_done(&repository, &["--yes", "--target", &target, "fixup.commit"]);
    let fixup = format!("> git commit --fixup={}", at(&hashes, 2));
    assert!(ended.out.contains(&fixup), "{}", ended.out);
    assert_eq!(subjects(&repository), ["Add e", "Add d", "Add c", "Add b", "Add a"]);
    assert_eq!(repository.git(&["show", "HEAD~2:a.txt"]), "a, fixed");
    assert_eq!(status(&repository), " M b.txt\n");
}

#[test]
fn fixup_commit__on_the_first_commit_rebases_from_the_root() {
    let (repository, hashes) = five();
    write(&repository, "a.txt", "a, fixed");
    let target = format!("commit:{}", at(&hashes, 0));
    let ended = cli_done(&repository, &["--yes", "--target", &target, "fixup.commit"]);
    assert!(ended.out.contains("> git add --all"), "{}", ended.out);
    assert!(ended.out.contains("--autostash --root"), "{}", ended.out);
    assert_eq!(repository.git(&["show", "HEAD~4:a.txt"]), "a, fixed");
    assert_eq!(status(&repository), "");
}
