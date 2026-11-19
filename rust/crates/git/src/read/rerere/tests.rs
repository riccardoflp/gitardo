use model::rerere::RerereStateRequest;
use testkit::Repository;

use super::{parse_state, read_state, rr_cache};
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
