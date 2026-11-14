use testkit::Repository;

use crate::{cli, cli_done};

#[test]
fn prints_what_is_checked_out_and_the_references() {
    let mut repository = Repository::new();
    let first = repository.commit("Add a", &[("a.txt", "a")]);
    repository.branch("feature");
    repository.tag("v1");
    let state = cli_done(&repository, &["state"]).out;
    let short = first.get(..7).unwrap_or_default();
    assert!(state.contains(&format!("HEAD        main at {short}")), "{state}");
    assert!(state.contains(&format!("* main  {short}")), "{state}");
    assert!(state.contains(&format!("  feature  {short}")), "{state}");
    assert!(state.contains(&format!("  v1  {short}")), "{state}");
    assert!(state.contains("Changes     none"), "{state}");
}

#[test]
fn says_when_the_folder_is_not_a_repository() {
    let repository = Repository::new();
    let outside = repository.path().join("..");
    let ended = cli(&repository, &["--repository", &outside.to_string_lossy(), "state"]);
    assert_eq!(ended.code, Some(1));
}

#[test]
fn says_what_is_wrong_with_the_command_line() {
    let repository = Repository::new();
    let ended = cli(&repository, &["--frobnicate"]);
    assert_eq!(ended.code, Some(2));
    assert!(ended.errors.contains("Usage:"), "{}", ended.errors);
}
