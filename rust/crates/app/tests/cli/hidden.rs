//! Hiding the changes of a file from git and showing them again, run by the engine on throwaway repositories
//! (`changes.skip`, `changes.unskip`, `changes.stop-tracking`).

use testkit::Repository;

use crate::{cli_done, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add the settings", &[("config/local.json", "{}\n"), ("a.txt", "a\n")]);
    write(&repository, "config/local.json", "{ \"port\": 1 }\n");
    repository
}

fn on_path(repository: &Repository, feature: &str, path: &str, answers: &[&str]) -> String {
    let target = format!("file:{path}");
    let mut arguments = vec!["--yes", "--target", target.as_str(), feature];
    arguments.extend(answers);
    cli_done(repository, &arguments).out
}

/// The letter `git ls-files -v` gives a file.
fn letter(repository: &Repository, path: &str) -> String {
    repository
        .git(&["ls-files", "-v", "--", path])
        .chars()
        .take(1)
        .collect()
}

#[test]
fn changes_skip__skip_it_in_the_working_tree_takes_the_file_out_of_git_status() {
    let repository = repository();
    let out = on_path(
        &repository,
        "changes.skip",
        "config/local.json",
        &["skip-how=skip-worktree"],
    );
    assert!(
        out.contains("> git update-index --skip-worktree -- config/local.json"),
        "{out}"
    );
    assert_eq!(repository.git(&["status", "--porcelain=v1"]), "");
    assert_eq!(letter(&repository, "config/local.json"), "S");
}

#[test]
fn changes_unskip__a_file_skipped_is_listed_as_modified_again_and_one_hidden_both_ways_is_marked_h() {
    let repository = repository();
    let _skip = repository.git(&["update-index", "--skip-worktree", "--", "config/local.json"]);
    let out = on_path(&repository, "changes.unskip", "config/local.json", &[]);
    assert!(
        out.contains("> git update-index --no-skip-worktree -- config/local.json"),
        "{out}"
    );
    assert_eq!(repository.git(&["status", "--porcelain=v1"]), " M config/local.json\n");
    let _skipped = repository.git(&["update-index", "--skip-worktree", "--", "a.txt"]);
    let _assumed = repository.git(&["update-index", "--assume-unchanged", "--", "a.txt"]);
    assert_eq!(letter(&repository, "a.txt"), "s");
    let _out = on_path(&repository, "changes.unskip", "a.txt", &[]);
    assert_eq!(letter(&repository, "a.txt"), "H");
}

#[test]
fn changes_stop_tracking__a_file_hidden_with_skip_worktree_is_out_of_the_index_after() {
    let repository = repository();
    let _skip = repository.git(&["update-index", "--skip-worktree", "--", "config/local.json"]);
    let out = on_path(&repository, "changes.stop-tracking", "config/local.json", &[]);
    assert!(
        out.contains("> git update-index --no-skip-worktree -- config/local.json"),
        "{out}"
    );
    assert_eq!(repository.git(&["ls-files", "--", "config/local.json"]), "");
}
