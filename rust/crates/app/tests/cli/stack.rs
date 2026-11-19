//! The other branches of a stack moved with a rebase, or left on the old commits (`rebase.update-refs`).

use testkit::Repository;

use crate::{cli_done, revision};

/// `p1`, `p2` and `p3` stacked one commit each on `main`, `p3` checked out, and `main` one commit further.
fn stack() -> Repository {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    for name in ["p1", "p2", "p3"] {
        repository.branch(name);
        repository.checkout(name);
        let file = format!("{name}.txt");
        let _part = repository.commit(&format!("Add {name}"), &[(file.as_str(), name)]);
    }
    repository.checkout("main");
    let _b = repository.commit("Add b", &[("b.txt", "b")]);
    repository.checkout("p3");
    repository
}

/// Whether `ancestor` is in the history of `descendant`.
fn is_ancestor(repository: &Repository, ancestor: &str, descendant: &str) -> bool {
    let merge_base = repository.git(&["merge-base", ancestor, descendant]);
    merge_base.trim() == revision(repository, ancestor)
}

#[test]
fn rebase_update_refs__rebasing_p3_onto_main_moves_p1_and_p2_and_says_so() {
    let repository = stack();
    let ended = cli_done(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert!(ended.out.contains("> git rebase --update-refs main"), "{}", ended.out);
    for name in ["p1", "p2"] {
        assert!(is_ancestor(&repository, "main", name), "{name} is on the new commits");
        assert!(is_ancestor(&repository, name, "p3"), "{name} is under p3");
    }
    assert!(ended.out.contains("`p1` and `p2` moved with it"), "{}", ended.out);
}

#[test]
fn rebase_update_refs__with_no_other_branch_the_command_has_neither_option() {
    let mut repository = Repository::new();
    let _a = repository.commit("Add a", &[("a.txt", "a")]);
    repository.branch("feature");
    let _b = repository.commit("Add b", &[("b.txt", "b")]);
    repository.checkout("feature");
    let _f = repository.commit("Add f", &[("f.txt", "f")]);
    let ended = cli_done(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert!(ended.out.contains("> git rebase main"), "{}", ended.out);
}

/// The hash of the commit a branch is on.
fn tip(repository: &Repository, name: &str) -> String {
    revision(repository, name)
}

#[test]
fn rebase_update_refs__with_p2_checked_out_in_another_worktree_it_stays_and_that_worktree_is_as_it_was() {
    let repository = stack();
    let before = tip(&repository, "p2");
    let folder = repository.path().with_file_name(format!(
        "{}-p2",
        repository
            .path()
            .file_name()
            .map_or_default(|name| name.to_string_lossy().into_owned())
    ));
    let folder_text = folder.to_string_lossy().into_owned();
    let _added = repository.git(&["worktree", "add", "--quiet", &folder_text, "p2"]);
    let asked = crate::cli(&repository, &["--target", "branch:main", "rebase.onto"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.out);
    let ended = cli_done(&repository, &["--yes", "--target", "branch:main", "rebase.onto"]);
    assert!(ended.out.contains("`p2` is checked out in"), "{}", ended.out);
    assert!(ended.out.contains("`p1` moved with it"), "{}", ended.out);
    assert_eq!(tip(&repository, "p2"), before, "p2 stays on the old commits");
    let there = |arguments: &[&str]| {
        let mut all = vec!["-C", folder_text.as_str()];
        all.extend_from_slice(arguments);
        repository.git(&all)
    };
    assert_eq!(
        there(&["status", "--porcelain"]),
        "",
        "its files and its index are as they were"
    );
    assert_eq!(there(&["rev-parse", "HEAD"]).trim_end(), before);
    let _removed = repository.git(&["worktree", "remove", "--force", &folder_text]);
}

#[test]
fn rebase_update_refs__editing_the_message_of_the_commit_p1_is_on_moves_p1_to_the_reworded_commit() {
    let repository = stack();
    let target = format!("commit:{}", tip(&repository, "p1"));
    let ended = cli_done(
        &repository,
        &["--yes", "--target", &target, "message.edit", "message=Add the part one"],
    );
    assert!(
        ended.out.contains("`p1` and `p2` moved with the rewrite"),
        "{}",
        ended.out
    );
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s", "p1"]).trim_end(),
        "Add the part one"
    );
    assert_eq!(tip(&repository, "p2^"), tip(&repository, "p1"));
    assert_eq!(tip(&repository, "p3^"), tip(&repository, "p2"));
}

#[test]
fn rebase_update_refs__squashing_the_commit_p2_is_on_into_the_one_before_moves_both_to_the_commit_they_become() {
    let repository = stack();
    let target = format!("commit:{}", tip(&repository, "p2"));
    let _ended = cli_done(
        &repository,
        &[
            "--yes",
            "--target",
            &target,
            "squash.into-previous",
            "subject=Add p1 and p2",
        ],
    );
    assert_eq!(
        repository.git(&["log", "-1", "--format=%s", "p1"]).trim_end(),
        "Add p1 and p2"
    );
    assert_eq!(tip(&repository, "p2"), tip(&repository, "p1"));
    assert_eq!(tip(&repository, "p3^"), tip(&repository, "p1"));
}

#[test]
fn rebase_update_refs__a_fixup_moves_the_branches_above_the_commit_it_changes() {
    let repository = stack();
    crate::write(&repository, "p1.txt", "p1 fixed");
    let _staged = repository.git(&["add", "p1.txt"]);
    let target = format!("commit:{}", tip(&repository, "p1"));
    let ended = cli_done(&repository, &["--yes", "--target", &target, "fixup.commit"]);
    assert!(ended.out.contains("--autostash --update-refs"), "{}", ended.out);
    assert_eq!(repository.git(&["show", "p1:p1.txt"]), "p1 fixed");
    assert_eq!(tip(&repository, "p2^"), tip(&repository, "p1"));
    assert_eq!(tip(&repository, "p3^"), tip(&repository, "p2"));
}
