//! The reads that version 1.13 adds for the details and the commit box of the extension: the signature of a commit, the
//! authors of the history, and how git signs, in the state.

use serde_json::{Value, json};

use crate::harness::repository;
use crate::reads::{opened, result};

#[test]
fn a_commit_signed_with_an_allowed_ssh_key_is_good_with_its_signer_and_one_not_signed_is_not_signed() {
    let mut repository = repository();
    let plain = repository.git(&["rev-parse", "HEAD"]).trim_end().to_owned();
    let key = repository.ssh_signing_key();
    repository.allow_signer(&key);
    let signed = repository.commit_signed("Sign the release");
    let mut engine = opened(&repository);
    let good = result(
        &mut engine,
        &repository,
        "read/details.read-signature",
        json!({ "commit": signed }),
    );
    assert_eq!(good["verdict"], json!("good"), "{good}");
    assert_eq!(good["kind"], json!("ssh"));
    assert_eq!(good["fingerprint"], json!(key.fingerprint));
    let none = result(
        &mut engine,
        &repository,
        "read/details.read-signature",
        json!({ "commit": plain }),
    );
    assert_eq!(none["verdict"], json!("notSigned"));
    assert_eq!(none["kind"], Value::Null);
}

#[test]
fn the_authors_of_the_history_come_with_how_many_commits_each_made_and_the_state_says_how_git_signs() {
    let repository = repository();
    let _ = repository.git(&["config", "commit.gpgSign", "true"]);
    let _ = repository.git(&["config", "gpg.format", "ssh"]);
    let _ = repository.git(&[
        "commit",
        "--quiet",
        "--no-gpg-sign",
        "--allow-empty",
        "--author=Ben <ben@example.com>",
        "-m",
        "Ben's commit",
    ]);
    let mut engine = opened(&repository);
    let read = result(&mut engine, &repository, "read/commit.read-authors", Value::Null);
    let authors = read["authors"].as_array().unwrap();
    assert_eq!(authors.len(), 2, "{read}");
    assert_eq!(authors[0]["commits"], json!(2));
    assert_eq!(authors[1]["email"], json!("ben@example.com"));
    assert_eq!(authors[1]["commits"], json!(1));
    let state = result(&mut engine, &repository, "read/repo.read-state", Value::Null);
    assert_eq!(
        state["signing"],
        json!({ "signsEveryCommit": true, "kind": "ssh", "key": null })
    );
}
