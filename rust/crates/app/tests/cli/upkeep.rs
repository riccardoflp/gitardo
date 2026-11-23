//! The upkeep of a repository: **Clean up…** with `git gc`, git's commit-graph file written, and git's scheduled
//! maintenance stopped. Every command runs
//! with the test's own configuration of git (`GIT_CONFIG_GLOBAL` and `HOME` in a throwaway folder), and
//! `git maintenance start`, which would schedule git in the system's scheduler, is never run: a repository is
//! registered with `git maintenance register`, which only writes that configuration.

use testkit::Repository;

use crate::{cli, cli_done};

/// A repository with a few commits, whose objects are all loose.
fn with_loose_objects() -> Repository {
    let mut repository = Repository::new();
    for index in 0..5 {
        let name = format!("file{index}.txt");
        let _ = repository.commit(&format!("Add {name}"), &[(name.as_str(), "some text\n")]);
    }
    repository
}

/// The number after `<key>: ` in `git count-objects -v`.
fn counted(repository: &Repository, key: &str) -> u64 {
    let output = repository.git(&["count-objects", "-v"]);
    output
        .lines()
        .find_map(|line| {
            line.strip_prefix(&format!("{key}: "))
                .and_then(|value| value.trim().parse().ok())
        })
        .unwrap_or(0)
}

/// The repositories `maintenance.repo` lists, in the test's own configuration.
fn registered(repository: &Repository) -> String {
    repository
        .command("git")
        .args(["config", "--global", "--get-all", "maintenance.repo"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

#[test]
fn upkeep_clean_up__it_asks_with_the_size_then_runs_gc_and_the_loose_objects_are_packed() {
    let repository = with_loose_objects();
    assert!(counted(&repository, "count") > 0);
    let asked = cli(&repository, &["upkeep.clean-up"]);
    assert_eq!(asked.code, Some(2), "it asks first: {}", asked.errors);
    let said = format!("{}{}", asked.out, asked.errors);
    assert!(said.contains("takes") && said.contains("loose objects"), "{said}");
    assert!(counted(&repository, "count") > 0, "cancelling runs nothing");
    let ended = cli_done(&repository, &["--yes", "upkeep.clean-up"]);
    assert!(ended.out.contains("git gc"), "{}", ended.out);
    assert_eq!(counted(&repository, "count"), 0, "the loose objects were packed");
    assert!(counted(&repository, "packs") >= 1);
}

#[test]
fn upkeep_start_background__it_asks_first_and_is_refused_on_a_repository_looked_after_already() {
    let repository = with_loose_objects();
    // Without --yes it stops at its question: `git maintenance start` does not run.
    let asked = cli(&repository, &["upkeep.start-background"]);
    assert_eq!(asked.code, Some(2), "{}", asked.errors);
    assert!(format!("{}{}", asked.out, asked.errors).contains("every hour"));
    assert_eq!(registered(&repository), "");
    let _ = repository.git(&["maintenance", "register"]);
    let refused = cli(&repository, &["--yes", "upkeep.start-background"]);
    assert_eq!(
        refused.code,
        Some(1),
        "it reads the repository as looked after: {}",
        refused.errors
    );
}

#[test]
fn upkeep_stop_background__it_unregisters_this_repository_and_leaves_another_registered() {
    let repository = with_loose_objects();
    let _ = repository.git(&["maintenance", "register"]);
    let _ = repository.git(&["config", "--global", "--add", "maintenance.repo", "/srv/another"]);
    let ended = cli_done(&repository, &["upkeep.stop-background"]);
    assert!(ended.out.contains("git maintenance unregister"), "{}", ended.out);
    let left = registered(&repository);
    assert_eq!(left.trim(), "/srv/another", "{left}");
    let refused = cli(&repository, &["upkeep.stop-background"]);
    assert_eq!(refused.code, Some(1), "no longer looked after: {}", refused.errors);
}

#[test]
fn upkeep_read_background__a_registered_repository_reads_as_looked_after_and_once_unregistered_not() {
    let repository = with_loose_objects();
    let _ = repository.git(&["maintenance", "register"]);
    let refused = cli(&repository, &["--yes", "upkeep.start-background"]);
    assert_eq!(refused.code, Some(1), "looked after: {}", refused.errors);
    let _ = repository.git(&["maintenance", "unregister"]);
    // Without --yes it stops at its question, which it asks only of a repository not looked after.
    let asked = cli(&repository, &["upkeep.start-background"]);
    assert_eq!(asked.code, Some(2), "not looked after: {}", asked.errors);
}

#[test]
fn upkeep_write_commit_graph__it_writes_the_file_with_the_changed_paths_and_is_then_no_longer_offered() {
    let repository = with_loose_objects();
    let file = repository.path().join(".git/objects/info/commit-graph");
    assert!(!file.exists());
    let ended = cli_done(&repository, &["upkeep.write-commit-graph"]);
    assert!(
        ended.out.contains("git commit-graph write --reachable --changed-paths"),
        "{}",
        ended.out
    );
    assert!(file.is_file(), "git wrote its commit-graph file");
    let request = git::read::upkeep::CommitGraphRequest {
        folder: repository.path().to_path_buf(),
        before: None,
    };
    let read = git::read::upkeep::read_commit_graph(&git::Git::new(repository.path()), &request).unwrap();
    assert!(read.has_file && !read.is_missing(), "{read:?}");
    let refused = cli(&repository, &["upkeep.write-commit-graph"]);
    assert_eq!(refused.code, Some(1), "no longer offered: {}", refused.errors);
}
