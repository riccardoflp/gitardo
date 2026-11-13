//! Editing the message of the last commit and of an older one.

use testkit::Repository;

use crate::{cli_done, revision, write};

#[test]
fn message_edit__the_last_commit_is_amended_from_standard_input_and_what_was_staged_stays_staged() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let last = repository.commit("Add b", &[("b.txt", "b\n")]);
    write(&repository, "a.txt", "staged\n");
    let _staged = repository.git(&["add", "a.txt"]);
    let target = format!("commit:{last}");
    let ended = cli_done(
        &repository,
        &["--target", &target, "message.edit", "message=Add the file b"],
    );
    assert!(
        ended.out.contains("> git commit --amend --only --allow-empty -F -\n"),
        "{}",
        ended.out
    );
    assert_eq!(
        repository.git(&["log", "-1", "--format=%B"]).trim_end(),
        "Add the file b"
    );
    assert_eq!(repository.git(&["diff", "--cached", "--name-only"]), "a.txt\n");
    assert_eq!(repository.git(&["show", "--name-only", "--format=", "HEAD"]), "b.txt\n");
}

#[test]
fn message_edit__the_third_commit_under_head_is_reworded_by_one_rebase_and_selected() {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n")]);
    let third = repository.commit("Add b", &[("b.txt", "b\n")]);
    let _second = repository.commit("Add c", &[("c.txt", "c\n")]);
    let _head = repository.commit("Add d", &[("d.txt", "d\n")]);
    write(&repository, "a.txt", "not committed\n");
    let target = format!("commit:{third}");
    let ended = cli_done(
        &repository,
        &[
            "--yes",
            "--target",
            &target,
            "message.edit",
            "message=Add the file b\n\nWith a body.",
        ],
    );
    let rebases = ended
        .out
        .lines()
        .filter(|line| line.contains(" rebase -i --autostash "))
        .count();
    assert_eq!(rebases, 1, "{}", ended.out);
    assert!(ended.out.contains("Select HEAD~2"), "{}", ended.out);
    let subjects = repository.git(&["log", "--format=%s", "-4"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        ["Add d", "Add c", "Add the file b", "Add a"]
    );
    assert_eq!(
        repository.git(&["log", "-1", "--format=%B", "HEAD~2"]).trim_end(),
        "Add the file b\n\nWith a body."
    );
    assert_ne!(revision(&repository, "HEAD~2"), third);
    assert_eq!(repository.git(&["status", "--porcelain"]), " M a.txt\n");
    let folders = std::fs::read_dir(repository.path().join(".git")).unwrap();
    assert!(
        folders
            .flatten()
            .all(|entry| !entry.file_name().to_string_lossy().starts_with("gitardo-rebase"))
    );
}
