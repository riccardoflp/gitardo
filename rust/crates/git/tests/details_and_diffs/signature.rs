//! `details.read-signature` against throwaway repositories whose commits are signed with an SSH key made for the test.

use std::fs;

use git::Git;
use git::read::details::{SignatureRequest, read_signature};
use model::CommitId;
use model::details::{CommitSignature, SignatureKind, SignatureVerdict};
use testkit::Repository;

/// Git in a repository, with the configuration of the tests alone, as the repository's own commands have it, and every
/// command it runs written to a trace file.
fn isolated_git(repository: &Repository) -> (Git, std::path::PathBuf) {
    let folder = repository.path().parent().unwrap();
    let trace = folder.join("trace.txt");
    let git = Git::new(repository.path())
        .with_variable("GIT_CONFIG_GLOBAL", folder.join("global.gitconfig"))
        .with_variable("GIT_CONFIG_NOSYSTEM", "1")
        .with_variable("GIT_TRACE", &trace);
    (git, trace)
}

/// The signature of a commit of a repository.
fn signature(repository: &Repository, commit: &str) -> CommitSignature {
    let (git, _) = isolated_git(repository);
    let request = SignatureRequest {
        commit: CommitId::parse(commit).unwrap(),
    };
    read_signature(&git, &request).unwrap()
}

#[test]
fn details_read_signature__a_commit_made_without_signing_is_not_signed_and_cat_file_ran_once() {
    let mut repository = Repository::new();
    let commit = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let (git, trace) = isolated_git(&repository);
    let request = SignatureRequest {
        commit: CommitId::parse(&commit).unwrap(),
    };
    let read = read_signature(&git, &request).unwrap();
    assert_eq!(read.verdict, SignatureVerdict::NotSigned);
    let traced = fs::read_to_string(trace).unwrap();
    let ran = |command: &str| traced.lines().filter(|line| line.contains(command)).count();
    assert_eq!(ran("git cat-file commit"), 1, "{traced}");
    assert_eq!(ran("git show"), 0, "{traced}");
}

#[test]
fn details_read_signature__a_commit_signed_with_ssh_and_allowed_signers_is_good_with_the_signer_and_fingerprint() {
    let mut repository = Repository::new();
    let key = repository.ssh_signing_key();
    repository.allow_signer(&key);
    let commit = repository.commit_signed("Sign the first commit");
    let read = signature(&repository, &commit);
    assert_eq!(read.verdict, SignatureVerdict::Good, "{read:?}");
    assert_eq!(read.kind, Some(SignatureKind::Ssh));
    assert_eq!(read.signer, "author@example.com");
    assert_eq!(read.fingerprint, key.fingerprint);
}

#[test]
fn details_read_signature__a_commit_signed_with_ssh_without_allowed_signers_is_signed_not_checked() {
    let mut repository = Repository::new();
    let _key = repository.ssh_signing_key();
    let commit = repository.commit_signed("Sign the first commit");
    let read = signature(&repository, &commit);
    assert_eq!(read.verdict, SignatureVerdict::SignedNotChecked, "{read:?}");
    assert_eq!(read.kind, Some(SignatureKind::Ssh));
}

#[test]
fn details_read_signature__a_signed_commit_whose_message_was_changed_afterwards_is_bad() {
    let mut repository = Repository::new();
    let key = repository.ssh_signing_key();
    repository.allow_signer(&key);
    let commit = repository.commit_signed("Sign the first commit");
    let object = repository.git(&["cat-file", "commit", &commit]);
    let changed = object.replace("Sign the first commit", "Sign another commit");
    let path = repository.path().parent().unwrap().join("changed-commit");
    fs::write(&path, changed).unwrap();
    let path = path.to_string_lossy().replace('\\', "/");
    let forged = repository.git(&["hash-object", "-t", "commit", "-w", &path]);
    let read = signature(&repository, forged.trim_end());
    assert_eq!(read.verdict, SignatureVerdict::Bad, "{read:?}");
}
