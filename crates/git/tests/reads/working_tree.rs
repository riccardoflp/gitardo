//! `changes.read-working-tree` and `identity.read` against throwaway repositories.

use std::fs;

use git::Git;
use git::read::changes::{WorkingTreeRequest, parse_working_tree, read_working_tree, working_tree_arguments};
use git::read::identity::{IdentityRequest, local_keys_arguments, parse_local_keys, read_identity};
use git::read::repo::{identity_arguments, parse_identity};
use model::state::ChangeKind;
use testkit::Repository;

use crate::golden::{Case, check, record};
use crate::{beside, run, write};

/// Awkward file names staged, changed in both, renamed, deleted and untracked.
fn awkward_repository() -> Repository {
    let mut repository = Repository::new();
    let files = [
        ("with space é.txt", "1\n"),
        ("pages/[id].tsx", "1\n"),
        ("-leading-dash.txt", "1\n"),
        ("renamed from.txt", "one\ntwo\nthree\nfour\n"),
    ];
    let _ = repository.commit("Add the files", &files);
    write(&repository, "pages/[id].tsx", "2\n");
    let _ = repository.git(&["add", "--", ":(literal)pages/[id].tsx"]);
    write(&repository, "pages/[id].tsx", "3\n");
    write(&repository, "with space é.txt", "2\n");
    let _ = repository.git(&["mv", "--", "renamed from.txt", "renamed to.txt"]);
    let _ = repository.git(&["rm", "--quiet", "--", "-leading-dash.txt"]);
    write(&repository, "new folder/[draft] ü.md", "draft\n");
    repository
}

#[test]
fn changes_read_working_tree__a_path_with_spaces_or_accents_comes_back_as_it_is_on_disk() {
    let repository = awkward_repository();
    let read = read_working_tree(&Git::new(repository.path()), &WorkingTreeRequest).unwrap();
    let unstaged: Vec<(&str, ChangeKind)> = read
        .unstaged
        .iter()
        .map(|change| (change.path.as_str(), change.kind))
        .collect();
    assert_eq!(
        unstaged,
        [
            ("pages/[id].tsx", ChangeKind::Modified),
            ("with space é.txt", ChangeKind::Modified)
        ]
    );
    assert_eq!(read.untracked, ["new folder/[draft] ü.md"]);
    let renamed = read
        .staged
        .iter()
        .find(|change| change.kind == ChangeKind::Renamed)
        .unwrap();
    assert_eq!(renamed.path, "renamed to.txt");
    assert_eq!(renamed.original_path.as_deref(), Some("renamed from.txt"));
    assert!(fs::exists(repository.path().join("with space é.txt")).unwrap());
}

#[test]
fn changes_read_working_tree__golden_awkward_names() {
    let case = Case {
        read: "working-tree",
        name: "awkward",
    };
    record(case, awkward_repository, &working_tree_arguments(&WorkingTreeRequest));
    check(case, |output| parse_working_tree(output).unwrap());
}

#[test]
fn identity_read__an_identity_set_only_globally_is_read_and_no_key_is_local() {
    let repository = Repository::new();
    let global = "[user]\n\tname = Zoë Global\n\temail = zoe@example.com\n";
    fs::write(beside(&repository).join("global.gitconfig"), global).unwrap();
    let mut words = vec!["-c", "core.quotepath=false"];
    let identity = identity_arguments();
    words.extend(identity.iter().map(|word| word.to_str().unwrap()));
    let identity = parse_identity(&run(&repository, &words).stdout).unwrap();
    assert_eq!(identity.name.as_deref(), Some("Zoë Global"));
    assert_eq!(identity.email.as_deref(), Some("zoe@example.com"));
    let local = local_keys_arguments();
    let local: Vec<&str> = local.iter().map(|word| word.to_str().unwrap()).collect();
    let output = run(&repository, &local);
    assert_eq!(output.status.code(), Some(1), "git ends with 1 when there is no key");
    assert_eq!(parse_local_keys(&output.stdout), Vec::<String>::new());
}

#[test]
fn identity_read__a_signing_key_set_in_the_repository_is_a_local_key() {
    let repository = Repository::new();
    let _ = repository.git(&["config", "user.signingkey", "ABCD1234"]);
    let _ = repository.git(&["config", "user.name", "Zoë Local"]);
    let read = read_identity(&Git::new(repository.path()), &IdentityRequest).unwrap();
    assert_eq!(read.local_keys, ["user.signingkey", "user.name"]);
    assert_eq!(read.identity.name.as_deref(), Some("Zoë Local"));
}

#[test]
fn identity_read__nothing_set_anywhere_is_empty_and_no_error() {
    let repository = Repository::new();
    for arguments in [identity_arguments(), local_keys_arguments()] {
        let words: Vec<&str> = arguments.iter().map(|word| word.to_str().unwrap()).collect();
        let output = run(&repository, &words);
        assert_eq!(output.status.code(), Some(1), "{words:?}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    }
    let identity = parse_identity(b"").unwrap();
    assert_eq!((identity.name, identity.email), (None, None));
    // Read as the program reads it, the repository sets no key of its own, whatever the person's configuration says.
    let read = read_identity(&Git::new(repository.path()), &IdentityRequest).unwrap();
    assert_eq!(read.local_keys, Vec::<String>::new());
}
