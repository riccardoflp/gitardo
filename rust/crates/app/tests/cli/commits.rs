//! What is copied of a commit, from `gitardo-cli`: it prints `Copy <text>` where the window puts the text on the
//! clipboard.

use testkit::Repository;

use super::{cli_done, revision};

#[test]
fn commits_copy_hash__gitardo_cli_copies_the_forty_characters_of_the_hash() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add a", &[("a.txt", "a\n")]);
    let hash = revision(&repository, "HEAD");
    let ended = cli_done(
        &repository,
        &["--target", &format!("commit:{hash}"), "commits.copy-hash"],
    );
    assert_eq!(hash.len(), 40);
    assert!(ended.out.contains(&format!("Copy {hash}\n")), "{}", ended.out);
}

#[test]
fn commits_copy_message__gitardo_cli_copies_the_message_as_git_prints_it_without_the_last_newline() {
    let mut repository = Repository::new();
    let message = "Fix the login\n\nThe session expired after a minute, which is far too early for anyone who reads \
                   a long page before submitting it.\n  An indented line.";
    let _ = repository.commit(message, &[("a.txt", "a\n")]);
    let printed = repository.git(&["log", "-1", "--format=%B"]);
    let hash = revision(&repository, "HEAD");
    let ended = cli_done(
        &repository,
        &["--target", &format!("commit:{hash}"), "commits.copy-message"],
    );
    let expected = printed.trim_end_matches('\n');
    assert_eq!(expected, message);
    assert!(ended.out.contains(&format!("Copy {expected}\n")), "{}", ended.out);
}
