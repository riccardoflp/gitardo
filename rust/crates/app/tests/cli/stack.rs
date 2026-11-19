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
