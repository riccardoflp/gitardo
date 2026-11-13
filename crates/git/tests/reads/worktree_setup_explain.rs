//! `worktree-setup.explain-change` on repositories built for each sentence: what git says of the path and of the
//! patterns before and after the change, and the sentence it gives.

use git::Git;
use git::read::worktree_setup::{PatternsRequest, read_path_status, read_what_patterns_take};
use model::worktree_setup::{
    ExplainInput, SetupAction, SetupChange, SetupPatterns, SetupPreview, change_patterns, explain_change,
};
use testkit::Repository;

use crate::write;

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

fn take(repository: &Repository, patterns: &SetupPatterns) -> SetupPreview {
    let request = PatternsRequest {
        folder: repository.path().to_path_buf(),
        link_patterns: patterns.link.clone(),
        copy_patterns: patterns.copy.clone(),
        commit: None,
    };
    read_what_patterns_take(&Git::new(repository.path()), &request).unwrap()
}

/// What a choice on a path does, with the link and copy patterns set before it.
fn explained(
    repository: &Repository,
    path: &str,
    action: SetupAction,
    (link, copy): (&[&str], &[&str]),
) -> SetupChange {
    let before_patterns = SetupPatterns {
        link: strings(link),
        copy: strings(copy),
    };
    let is_folder = repository.path().join(path).is_dir();
    let after_patterns = change_patterns(&before_patterns, (path, is_folder), action);
    let (tracked, ignored) = read_path_status(&Git::new(repository.path()), path).unwrap();
    explain_change(&ExplainInput {
        path,
        action,
        tracked,
        ignored,
        before: &take(repository, &before_patterns),
        after: &take(repository, &after_patterns),
    })
}

/// A repository that ignores dependencies, settings of this machine and a local configuration file, with a tracked
/// readme, a tracked file in `config` and in `src`, and something of each on disk.
fn repository() -> Repository {
    let mut repository = Repository::new();
    let files = [
        (".gitignore", "node_modules/\n.env\n*.local.json\nconfig/local.json\n"),
        ("README.md", "Read me\n"),
        ("config/app.json", "{}\n"),
        ("src/main.rs", "fn main() {}\n"),
    ];
    let _ = repository.commit("Add the project", &files);
    write(&repository, "node_modules/left-pad/index.js", "1\n");
    write(&repository, "node_modules/x.local.json", "{}\n");
    write(&repository, "packages/api/node_modules/b/index.js", "1\n");
    write(&repository, ".env", "KEY=1\n");
    write(&repository, "config/local.json", "{}\n");
    write(&repository, "notes/todo.txt", "1\n");
    write(&repository, "notes.txt", "1\n");
    write(&repository, "mixed/app.local.json", "{}\n");
    write(&repository, "mixed/plain.txt", "1\n");
    repository
}

fn sentence(change: &SetupChange) -> &str {
    &change.sentence
}

#[test]
fn worktree_setup_explain_change__each_sentence_is_given_in_its_case() {
    let repository = repository();
    let link = |path: &str, link: &[&str]| explained(&repository, path, SetupAction::Link, (link, &[]));
    let cases = [
        (
            link("node_modules", &["node_modules/"]),
            "node_modules is linked into new worktrees already, by the pattern node_modules/.",
        ),
        (
            link("packages/api/node_modules", &[]),
            "packages/api/node_modules will be linked into new worktrees.",
        ),
        (
            link("src", &[]),
            "Git tracks files in src: each worktree has them from its commit.",
        ),
        (
            link("notes", &[]),
            "Git does not ignore notes: only what it ignores is linked or copied into new worktrees.",
        ),
        (
            link("mixed", &[]),
            "Git ignores only part of mixed: only what it ignores is linked or copied into new worktrees.",
        ),
    ];
    for (change, expected) in &cases {
        assert_eq!(sentence(change), *expected);
    }
    assert_copy_sentences(&repository);
    let still = explained(
        &repository,
        "node_modules",
        SetupAction::StopLinking,
        (&["/node_modules/", "node_modules/"], &[]),
    );
    assert_eq!(
        sentence(&still),
        "node_modules is still linked into new worktrees, by the pattern node_modules/."
    );
    let gone = explained(&repository, ".env", SetupAction::StopCopying, (&[], &["/.env"]));
    assert_eq!(sentence(&gone), "New worktrees will no longer get .env.");
}

/// The sentences of **Copy into New Worktrees**, each in its case.
fn assert_copy_sentences(repository: &Repository) {
    let copy = |path: &str, link: &[&str], copy: &[&str]| explained(repository, path, SetupAction::Copy, (link, copy));
    let cases = [
        (
            copy(".env", &[], &["/.env"]),
            ".env is copied into new worktrees already, by the pattern /.env.",
        ),
        (copy(".env", &[], &[]), ".env will be copied into new worktrees."),
        (
            copy("config", &[], &[]),
            "What git ignores in config will be copied into new worktrees.",
        ),
        (
            copy("node_modules", &["node_modules/"], &[]),
            "node_modules is linked into new worktrees by the pattern node_modules/, which comes before copying.",
        ),
        (
            copy("node_modules/x.local.json", &["node_modules/"], &[]),
            "node_modules/x.local.json is in node_modules, which the pattern node_modules/ links into new worktrees: nothing in it is copied.",
        ),
        (
            copy("README.md", &[], &[]),
            "Git tracks README.md: each worktree has it from its commit.",
        ),
        (
            copy("notes.txt", &[], &[]),
            "Git does not ignore notes.txt: only what it ignores is linked or copied into new worktrees.",
        ),
    ];
    for (change, expected) in &cases {
        assert_eq!(sentence(change), *expected);
    }
}

#[test]
fn worktree_setup_explain_change__only_will_be_and_still_write_the_settings() {
    let repository = repository();
    let writes = |path: &str, action: SetupAction, (link, copy): (&[&str], &[&str])| {
        explained(&repository, path, action, (link, copy)).writes
    };
    assert!(writes("packages/api/node_modules", SetupAction::Link, (&[], &[])));
    assert!(writes(".env", SetupAction::Copy, (&[], &[])));
    assert!(writes("config", SetupAction::Copy, (&[], &[])));
    assert!(writes(
        "node_modules",
        SetupAction::StopLinking,
        (&["/node_modules/", "node_modules/"], &[])
    ));
    assert!(!writes("node_modules", SetupAction::Link, (&["node_modules/"], &[])));
    assert!(!writes("src", SetupAction::Link, (&[], &[])));
    assert!(!writes("notes", SetupAction::Link, (&[], &[])));
    assert!(!writes("README.md", SetupAction::Copy, (&[], &[])));
    assert!(!writes("node_modules", SetupAction::Copy, (&["node_modules/"], &[])));
}
