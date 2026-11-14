use testkit::Repository;

use crate::{cli, cli_done, revision, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a")]);
    repository
}

#[test]
fn commit_create__commits_what_is_staged_with_the_message() {
    let repository = repository();
    write(&repository, "a.txt", "changed");
    let _staged = repository.git(&["add", "a.txt"]);
    let ended = cli_done(&repository, &["commit.create", "message=Fix the parser"]);
    assert!(
        ended.out.contains("> git commit -m \"Fix the parser\""),
        "{}",
        ended.out
    );
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s"]).trim_end(),
        "Fix the parser"
    );
}

#[test]
fn commit_create__stages_all_after_confirmation_with_untracked_files() {
    let repository = repository();
    write(&repository, "a.txt", "changed");
    write(&repository, "new.txt", "new");
    let ended = cli_done(&repository, &["commit.create", "message=Change all", "stage-all=yes"]);
    assert!(ended.out.contains("> git add --all\n"), "{}", ended.out);
    let files = repository.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(files.lines().collect::<Vec<_>>(), ["a.txt", "new.txt"]);
}

#[test]
fn commit_create__declining_runs_nothing() {
    let repository = repository();
    let before = revision(&repository, "HEAD");
    write(&repository, "a.txt", "changed");
    let ended = cli(&repository, &["commit.create", "message=Change all", "stage-all=no"]);
    assert_eq!(ended.code, Some(2));
    assert!(
        ended.errors.contains("Stage all changes and commit them?"),
        "{}",
        ended.errors
    );
    assert_eq!(revision(&repository, "HEAD"), before);
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "");
}

#[test]
fn commit_create__refuses_a_message_of_spaces_and_blank_lines() {
    let repository = repository();
    write(&repository, "a.txt", "changed");
    let _staged = repository.git(&["add", "a.txt"]);
    let ended = cli(&repository, &["commit.create", "message=  \n  "]);
    assert_eq!(ended.code, Some(1));
    assert!(ended.errors.contains("Enter a commit message."), "{}", ended.errors);
}

#[test]
fn commit_amend__a_new_message_keeps_the_files() {
    let repository = repository();
    let files_before = repository.git(&["show", "--name-only", "--format=", "HEAD"]);
    let ended = cli_done(&repository, &["commit.amend", "message=Better words"]);
    assert!(
        ended.out.contains("> git commit --amend -m \"Better words\""),
        "{}",
        ended.out
    );
    assert_eq!(repository.git(&["log", "-1", "--format=%s"]).trim_end(), "Better words");
    assert_eq!(
        repository.git(&["show", "--name-only", "--format=", "HEAD"]),
        files_before
    );
    assert_eq!(repository.git(&["rev-list", "--count", "HEAD"]).trim_end(), "1");
}

#[test]
fn commit_amend__an_empty_message_keeps_the_old_one_and_takes_what_is_staged() {
    let repository = repository();
    write(&repository, "b.txt", "b");
    let _staged = repository.git(&["add", "b.txt"]);
    let ended = cli_done(&repository, &["commit.amend", "message="]);
    assert!(ended.out.contains("> git commit --amend --no-edit"), "{}", ended.out);
    assert_eq!(repository.git(&["log", "-1", "--format=%s"]).trim_end(), "Add a");
    let files = repository.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(files.lines().collect::<Vec<_>>(), ["a.txt", "b.txt"]);
}
