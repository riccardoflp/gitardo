use model::rerere::{MarkedRequest, RerereStateRequest, ResolvedAsBefore, ResolvedRequest};
use testkit::Repository;

use super::{marked_arguments, parse_state, read_marked, read_resolved, read_state, rr_cache};
use crate::Git;

/// Git in a repository, with no configuration but the repository's own.
fn git_of(repository: &Repository) -> Git {
    let empty = repository.path().join(".git").join("empty-config");
    std::fs::write(&empty, "").unwrap();
    Git::new(repository.path())
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_CONFIG_GLOBAL", empty.as_os_str())
}

/// What the read gives in a repository.
fn state_of(repository: &Repository) -> model::rerere::RerereState {
    let request = RerereStateRequest {
        git_folder: repository.path().join(".git"),
    };
    read_state(&git_of(repository), &request).unwrap()
}

#[test]
fn rerere_read_state__enabled_true_is_on_and_false_is_off_even_with_an_rr_cache_folder() {
    let repository = Repository::new();
    let _ = repository.git(&["config", "rerere.enabled", "true"]);
    assert!(state_of(&repository).is_on);
    let _ = repository.git(&["config", "rerere.enabled", "false"]);
    std::fs::create_dir_all(repository.path().join(".git").join("rr-cache")).unwrap();
    assert!(!state_of(&repository).is_on);
}

#[test]
fn rerere_read_state__with_nothing_set_an_rr_cache_folder_is_on_and_neither_is_off() {
    let repository = Repository::new();
    assert!(!state_of(&repository).is_on);
    std::fs::create_dir_all(repository.path().join(".git").join("rr-cache")).unwrap();
    assert!(state_of(&repository).is_on);
    // A linked worktree finds the folder of the repository through its file `commondir`.
    let linked = repository.path().join(".git").join("worktrees").join("other");
    std::fs::create_dir_all(&linked).unwrap();
    std::fs::write(linked.join("commondir"), "../..\n").unwrap();
    assert!(rr_cache(&linked).is_dir());
}

#[test]
fn rerere_read_state__auto_update_true_reads_as_staging_what_it_writes() {
    let repository = Repository::new();
    let _ = repository.git(&["config", "rerere.autoUpdate", "true"]);
    let state = state_of(&repository);
    assert!(state.stages);
    assert!(state.sets_stages);
    assert_eq!(
        parse_state("rerere.enabled true\nrerere.autoupdate false\n"),
        (Some(true), Some(false))
    );
}

/// A repository with rerere on whose `m.txt` and `n.txt` changed on `main` and on `topic`, in which git recorded how
/// `m.txt` alone was resolved, with `rerere.autoUpdate` set as asked; `main` checked out, nothing merged.
fn recorded(stages: bool) -> Repository {
    let mut repository = Repository::new();
    let _ = repository.git(&["config", "rerere.enabled", "true"]);
    let _ = repository.git(&["config", "rerere.autoUpdate", if stages { "true" } else { "false" }]);
    let _ = repository.commit("Add the files", &[("m.txt", "base m\n"), ("n.txt", "base n\n")]);
    repository.branch("topic");
    let _ = repository.commit("Change them on main", &[("m.txt", "main m\n"), ("n.txt", "main n\n")]);
    repository.checkout("topic");
    let _ = repository.commit(
        "Change them on topic",
        &[("m.txt", "topic m\n"), ("n.txt", "topic n\n")],
    );
    repository.checkout("main");
    merge_topic(&repository);
    std::fs::write(repository.path().join("m.txt"), "resolved m\n").unwrap();
    let _ = repository.git(&["rerere"]);
    let _ = repository.git(&["merge", "--abort"]);
    repository
}

/// Merges `topic`, which stops on conflicts.
fn merge_topic(repository: &Repository) {
    let output = repository
        .command("git")
        .args(["merge", "--no-edit", "topic"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

/// The files git resolved as before, read as the repository read asks.
fn resolved_of(repository: &Repository) -> Vec<ResolvedAsBefore> {
    let unmerged = repository.git(&["diff", "--name-only", "--diff-filter=U"]);
    let request = ResolvedRequest {
        git_folder: repository.path().join(".git"),
        conflicted: unmerged.lines().map(str::to_owned).collect(),
    };
    read_resolved(&git_of(repository), &request).unwrap()
}

/// A file resolved as before, staged or not.
fn resolved(path: &str, is_staged: bool) -> ResolvedAsBefore {
    ResolvedAsBefore {
        path: path.to_owned(),
        is_staged,
    }
}

#[test]
fn rerere_read_resolved__a_merge_stopped_on_m_and_n_with_a_resolution_for_m_alone_gives_m() {
    let repository = recorded(false);
    merge_topic(&repository);
    assert_eq!(resolved_of(&repository), [resolved("m.txt", false)]);
    let m = std::fs::read_to_string(repository.path().join("m.txt")).unwrap();
    assert_eq!(m, "resolved m\n");
}

#[test]
fn rerere_read_resolved__with_auto_update_the_same_merge_gives_m_which_is_staged() {
    let repository = recorded(true);
    merge_topic(&repository);
    assert_eq!(resolved_of(&repository), [resolved("m.txt", true)]);
    let staged = repository.git(&["diff", "--cached", "--name-only", "--diff-filter=M"]);
    assert!(staged.lines().any(|path| path == "m.txt"), "{staged}");
}

/// The files git resolved as before, as the read of the repository gives them.
fn read_with_the_repository(repository: &Repository) -> model::state::RepositoryState {
    let request = crate::read::repo::StateRequest {
        folder: repository.path().to_path_buf(),
    };
    crate::read::repo::read_state(&git_of(repository), &request).unwrap()
}

#[test]
fn rerere_read_resolved__with_rerere_off_the_read_of_the_repository_runs_nothing_and_gives_nothing() {
    let repository = recorded(false);
    let _ = repository.git(&["config", "rerere.enabled", "false"]);
    merge_topic(&repository);
    let state = read_with_the_repository(&repository);
    assert!(!state.rerere.is_on);
    assert_eq!(state.resolved_as_before, Vec::<ResolvedAsBefore>::new());
}

#[test]
fn rerere_read_resolved__once_the_merge_is_committed_or_aborted_it_gives_nothing() {
    let repository = recorded(false);
    merge_topic(&repository);
    assert_eq!(read_with_the_repository(&repository).resolved_as_before.len(), 1);
    let _ = repository.git(&["merge", "--abort"]);
    assert_eq!(
        read_with_the_repository(&repository).resolved_as_before,
        Vec::<ResolvedAsBefore>::new()
    );
    merge_topic(&repository);
    std::fs::write(repository.path().join("n.txt"), "resolved n\n").unwrap();
    let _ = repository.git(&["add", "m.txt", "n.txt"]);
    let _ = repository.git(&["commit", "--no-edit", "--quiet"]);
    assert_eq!(
        read_with_the_repository(&repository).resolved_as_before,
        Vec::<ResolvedAsBefore>::new()
    );
}

/// What git marked in `m.txt` when the merge stopped, made again.
fn marked_m(repository: &Repository) -> String {
    let request = MarkedRequest {
        path: String::from("m.txt"),
        current: String::from("main"),
        incoming: String::from("topic"),
    };
    String::from_utf8(read_marked(&git_of(repository), &request).unwrap()).unwrap()
}

#[test]
fn rerere_read_marked__for_m_resolved_as_before_the_text_has_the_markers_and_both_sides_named() {
    let repository = recorded(false);
    merge_topic(&repository);
    assert_eq!(
        marked_m(&repository),
        "<<<<<<< main\nmain m\n=======\ntopic m\n>>>>>>> topic\n"
    );
}

#[test]
fn rerere_read_marked__the_exit_code_1_of_one_conflict_is_read_as_success() {
    let repository = recorded(false);
    merge_topic(&repository);
    let request = MarkedRequest {
        path: String::from("m.txt"),
        current: String::from("main"),
        incoming: String::from("topic"),
    };
    let output = git_of(&repository)
        .read_output(&marked_arguments(&request, ":1:m.txt"))
        .unwrap();
    assert_eq!(output.code, Some(1));
    assert!(marked_m(&repository).contains("======="));
}

#[test]
fn rerere_read_marked__nothing_in_the_working_tree_or_the_index_changes() {
    let repository = recorded(false);
    merge_topic(&repository);
    let status = repository.git(&["status", "--porcelain=v2"]);
    let _ = marked_m(&repository);
    assert_eq!(repository.git(&["status", "--porcelain=v2"]), status);
    let m = std::fs::read_to_string(repository.path().join("m.txt")).unwrap();
    assert_eq!(m, "resolved m\n");
}
