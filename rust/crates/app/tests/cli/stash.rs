use testkit::{Remote, Repository};

use crate::{checked_out, cli, cli_done, revision, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit(
        "Add the files",
        &[("a.txt", "a\n"), ("b.txt", "b\n"), ("src/app.rs", "app\n")],
    );
    repository
}

fn stashes(repository: &Repository) -> String {
    repository.git(&["stash", "list", "--format=%gd %gs"])
}

fn status(repository: &Repository) -> String {
    repository.git(&["status", "--porcelain"])
}

#[test]
fn stash_push__takes_changed_staged_and_untracked_files() {
    let repository = repository();
    write(&repository, "a.txt", "a, changed\n");
    write(&repository, "b.txt", "b, staged\n");
    let _staged = repository.git(&["add", "b.txt"]);
    write(&repository, "new.txt", "new\n");
    let ended = cli_done(&repository, &["stash.push", "message="]);
    assert!(
        ended.out.contains("> git stash push --include-untracked\n"),
        "{}",
        ended.out
    );
    assert_eq!(status(&repository), "");
}

#[test]
fn stash_push__a_message_typed_names_the_stash_and_git_names_one_without() {
    let repository = repository();
    write(&repository, "a.txt", "a, changed\n");
    let _done = cli_done(&repository, &["stash.push", "message=before the merge"]);
    assert!(stashes(&repository).contains("stash@{0} On main: before the merge"));
    write(&repository, "a.txt", "a, changed again\n");
    let _done = cli_done(&repository, &["stash.push", "message="]);
    assert!(
        stashes(&repository).contains("stash@{0} WIP on main: "),
        "{}",
        stashes(&repository)
    );
}

#[test]
fn stash_push__backing_out_of_the_box_stashes_nothing() {
    let repository = repository();
    write(&repository, "a.txt", "a, changed\n");
    let ended = cli(&repository, &["stash.push"]);
    assert_eq!(ended.code, Some(2));
    assert_eq!(stashes(&repository), "");
}

#[test]
fn stash_push_staged__stashes_only_the_staged_file() {
    let repository = repository();
    write(&repository, "a.txt", "a, staged\n");
    let _staged = repository.git(&["add", "a.txt"]);
    write(&repository, "b.txt", "b, changed\n");
    let _done = cli_done(&repository, &["stash.push-staged", "message="]);
    assert_eq!(status(&repository), " M b.txt\n");
}

#[test]
fn stash_push_file__stashes_one_file_staged_and_unstaged_alike() {
    let repository = repository();
    write(&repository, "src/app.rs", "app, staged\n");
    let _staged = repository.git(&["add", "src/app.rs"]);
    write(&repository, "src/app.rs", "app, staged and changed\n");
    write(&repository, "a.txt", "a, changed\n");
    let _done = cli_done(
        &repository,
        &["--target", "files:src/app.rs", "stash.push-file", "message="],
    );
    assert_eq!(status(&repository), " M a.txt\n");
}

#[test]
fn stash_push_file__an_untracked_file_goes_into_the_stash() {
    let repository = repository();
    write(&repository, "notes.txt", "mine\n");
    let _done = cli_done(
        &repository,
        &["--target", "files:notes.txt", "stash.push-file", "message="],
    );
    assert!(!repository.path().join("notes.txt").exists());
    assert!(stashes(&repository).contains("stash@{0}"));
}

#[test]
fn stash_push_file__two_files_ticked_are_the_only_ones_stashed() {
    let repository = repository();
    for file in ["a.txt", "b.txt", "src/app.rs"] {
        write(&repository, file, "changed\n");
    }
    let _done = cli_done(
        &repository,
        &["--target", "files:a.txt,b.txt", "stash.push-file", "message="],
    );
    assert_eq!(status(&repository), " M src/app.rs\n");
}

#[test]
fn stash_push_file__a_name_that_is_a_pattern_leaves_its_lookalikes() {
    let mut repository = repository();
    let _pages = repository.commit(
        "Add the pages",
        &[
            ("pages/[id].tsx", "1\n"),
            ("pages/i.tsx", "2\n"),
            ("pages/d.tsx", "3\n"),
        ],
    );
    for file in ["pages/[id].tsx", "pages/i.tsx", "pages/d.tsx"] {
        write(&repository, file, "changed\n");
    }
    let _done = cli_done(
        &repository,
        &["--target", "files:pages/[id].tsx", "stash.push-file", "message="],
    );
    assert_eq!(status(&repository), " M pages/d.tsx\n M pages/i.tsx\n");
}

#[test]
fn stash_push_file__a_staged_rename_stashes_both_names() {
    let repository = repository();
    let _moved = repository.git(&["mv", "a.txt", "renamed.txt"]);
    let _done = cli_done(
        &repository,
        &["--target", "files:renamed.txt", "stash.push-file", "message="],
    );
    assert_eq!(status(&repository), "");
    assert!(repository.path().join("a.txt").exists());
}

/// A repository with one stash of a change to `a.txt`.
fn stashed() -> Repository {
    let repository = repository();
    write(&repository, "a.txt", "a, stashed\n");
    let _stashed = repository.git(&["stash", "push", "--quiet", "-m", "try this"]);
    repository
}

/// A repository whose stash changes the line of `a.txt` that a commit made after it changed too.
fn stashed_then_conflicting() -> Repository {
    let mut repository = stashed();
    let _later = repository.commit("Change a", &[("a.txt", "a, committed\n")]);
    repository
}

#[test]
fn stash_apply__asks_first_with_the_note_also_with_preview_off() {
    let repository = stashed();
    let asked = cli(&repository, &["--target", "stash:0", "stash.apply"]);
    assert_eq!(asked.code, Some(2));
    assert!(
        asked
            .out
            .contains("Its changes go into the files, and it stays in the list."),
        "{}",
        asked.out
    );
    assert!(asked.out.contains("No conflicts expected."), "{}", asked.out);
}

#[test]
fn stash_apply__puts_the_changes_in_the_files_and_keeps_the_stash() {
    let repository = stashed();
    let _done = cli_done(&repository, &["--yes", "--target", "stash:0", "stash.apply"]);
    assert_eq!(status(&repository), " M a.txt\n");
    assert!(stashes(&repository).contains("try this"));
}

#[test]
fn stash_apply__a_conflict_leaves_the_files_in_conflict_and_keeps_the_stash() {
    let repository = stashed_then_conflicting();
    let asked = cli(&repository, &["--target", "stash:0", "stash.apply"]);
    assert!(
        asked.out.contains("This will stop with conflicts in 1 file:"),
        "{}",
        asked.out
    );
    let ended = cli(&repository, &["--yes", "--target", "stash:0", "stash.apply"]);
    assert_eq!(ended.code, Some(1));
    assert!(cli_done(&repository, &["state"]).out.contains("1 in conflict"));
    assert!(stashes(&repository).contains("try this"));
}

#[test]
fn stash_apply__git_refuses_over_a_changed_file_and_nothing_changes() {
    let repository = stashed();
    write(&repository, "a.txt", "a, mine\n");
    let asked = cli(&repository, &["--target", "stash:0", "stash.apply"]);
    assert!(
        asked.out.contains("Git will refuse it and change nothing"),
        "{}",
        asked.out
    );
    let ended = cli(&repository, &["--yes", "--target", "stash:0", "stash.apply"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("a.txt"),
        "git's words are shown: {}",
        ended.errors
    );
    assert_eq!(
        std::fs::read_to_string(repository.path().join("a.txt")).unwrap_or_default(),
        "a, mine\n"
    );
}

#[test]
fn stash_pop__a_clean_pop_drops_the_stash() {
    let repository = stashed();
    let _done = cli_done(&repository, &["--yes", "--target", "stash:0", "stash.pop"]);
    assert_eq!(status(&repository), " M a.txt\n");
    assert_eq!(stashes(&repository), "");
}

#[test]
fn stash_pop__a_conflict_keeps_the_stash() {
    let repository = stashed_then_conflicting();
    let ended = cli(&repository, &["--yes", "--target", "stash:0", "stash.pop"]);
    assert_eq!(ended.code, Some(1));
    assert!(cli_done(&repository, &["state"]).out.contains("1 in conflict"));
    assert!(stashes(&repository).contains("try this"));
}

#[test]
fn stash_pop__asks_first_also_with_preview_off() {
    let repository = stashed();
    let asked = cli(&repository, &["--target", "stash:0", "stash.pop"]);
    assert_eq!(asked.code, Some(2));
    assert!(asked.out.contains("Pop stash@{0}"), "{}", asked.out);
}

#[test]
fn stash_drop__asks_with_a_warning_then_renumbers() {
    let repository = stashed();
    write(&repository, "b.txt", "b, stashed\n");
    let _second = repository.git(&["stash", "push", "--quiet", "-m", "newer"]);
    let asked = cli(&repository, &["--target", "stash:0", "stash.drop"]);
    assert_eq!(asked.code, Some(2));
    assert!(
        asked.out.contains("The stashed changes will be deleted."),
        "{}",
        asked.out
    );
    let _done = cli_done(&repository, &["--yes", "--target", "stash:0", "stash.drop"]);
    assert_eq!(stashes(&repository), "stash@{0} On main: try this\n");
}

#[test]
fn stash_branch__makes_the_branch_where_the_stash_was_made_and_applies_it() {
    let mut repository = stashed();
    let made_on = revision(&repository, "stash@{0}^1");
    let _later = repository.commit("Add c", &[("c.txt", "c\n")]);
    let _done = cli_done(&repository, &["--target", "stash:0", "stash.branch", "name=rescue"]);
    assert_eq!(checked_out(&repository), "rescue");
    assert_eq!(revision(&repository, "HEAD"), made_on);
    assert_eq!(status(&repository), " M a.txt\n");
}

#[test]
fn stash_branch__the_stash_leaves_the_list_once_applied_cleanly() {
    let repository = stashed();
    let _done = cli_done(&repository, &["--target", "stash:0", "stash.branch", "name=rescue"]);
    assert_eq!(stashes(&repository), "");
}

#[test]
fn stash_branch__a_taken_name_or_one_git_refuses_is_not_accepted() {
    let repository = stashed();
    for name in ["name=main", "name=bad..name"] {
        let ended = cli(&repository, &["--target", "stash:0", "stash.branch", name]);
        assert_eq!(ended.code, Some(1), "{name}");
    }
    assert!(stashes(&repository).contains("try this"));
}

/// A pull with `--autostash` whose incoming commit changes the line changed in the working tree.
fn autostash_conflict() -> Repository {
    let mut repository = repository();
    let _remote = Remote::new(&repository);
    let base = revision(&repository, "HEAD");
    let _theirs = repository.commit("Change a", &[("a.txt", "a, theirs\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    write(&repository, "a.txt", "a, mine\n");
    repository
}

#[test]
fn stash_autostash_left__a_pull_whose_changes_no_longer_fit_warns_of_the_stash() {
    let repository = autostash_conflict();
    let ended = cli_done(&repository, &["sync.pull"]);
    assert!(
        ended
            .out
            .contains("Pulled, but your changes do not fit on what came in."),
        "{}",
        ended.out
    );
    assert!(stashes(&repository).to_lowercase().contains("autostash"));
    assert!(cli_done(&repository, &["state"]).out.contains("1 in conflict"));
}

#[test]
fn stash_autostash_left__a_pull_whose_changes_go_back_says_nothing() {
    let mut repository = repository();
    let _remote = Remote::new(&repository);
    let base = revision(&repository, "HEAD");
    let _theirs = repository.commit("Change b", &[("b.txt", "b, theirs\n")]);
    let _pushed = repository.git(&["push", "--quiet", "origin", "main"]);
    let _back = repository.git(&["reset", "--quiet", "--hard", &base]);
    write(&repository, "a.txt", "a, mine\n");
    let ended = cli_done(&repository, &["sync.pull"]);
    assert!(!ended.out.contains("Pulled, but"), "{}", ended.out);
    assert_eq!(stashes(&repository), "");
}
