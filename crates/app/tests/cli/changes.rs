//! Staging, unstaging and discarding run by the engine on throwaway repositories, and what git says after.

use testkit::{CONFLICT_FILE, Repository};

use crate::{cli_done, write};

fn repository() -> Repository {
    let mut repository = Repository::new();
    let _first = repository.commit("Add a", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    repository
}

/// What `git status --porcelain` says, one line per file.
fn status(repository: &Repository) -> Vec<String> {
    repository
        .git(&["status", "--porcelain=v1", "--untracked-files=all"])
        .lines()
        .map(str::to_owned)
        .collect()
}

fn on_file(repository: &Repository, feature: &str, path: &str) -> String {
    let target = format!("file:{path}");
    cli_done(repository, &["--yes", "--target", &target, feature]).out
}

#[test]
fn changes_stage__an_untracked_file_is_staged_once_run() {
    let repository = repository();
    write(&repository, "new.txt", "new\n");
    let out = on_file(&repository, "changes.stage", "new.txt");
    assert!(out.contains("> git add -- new.txt"), "{out}");
    assert_eq!(status(&repository), ["A  new.txt"]);
}

#[test]
fn changes_stage__a_file_partly_staged_is_staged_whole() {
    let repository = repository();
    write(&repository, "a.txt", "a\nb\n");
    let _staged = repository.git(&["add", "a.txt"]);
    write(&repository, "a.txt", "a\nb\nc\n");
    let _out = on_file(&repository, "changes.stage", "a.txt");
    assert_eq!(status(&repository), ["M  a.txt"]);
}

#[test]
fn changes_stage__a_deleted_file_has_its_deletion_staged() {
    let repository = repository();
    std::fs::remove_file(repository.path().join("b.txt")).unwrap();
    let _out = on_file(&repository, "changes.stage", "b.txt");
    assert_eq!(status(&repository), ["D  b.txt"]);
}

#[test]
fn changes_pathspec__staging_a_name_with_brackets_leaves_the_names_it_would_match_unstaged() {
    let repository = repository();
    std::fs::create_dir(repository.path().join("pages")).unwrap();
    for name in ["[id].tsx", "i.tsx", "d.tsx"] {
        write(&repository, &format!("pages/{name}"), "page\n");
    }
    let out = on_file(&repository, "changes.stage", "pages/[id].tsx");
    assert!(out.contains("> git add -- :(literal)pages/[id].tsx"), "{out}");
    assert_eq!(
        status(&repository),
        ["A  pages/[id].tsx", "?? pages/d.tsx", "?? pages/i.tsx"]
    );
}

#[test]
fn changes_unstage__the_file_keeps_its_changes_on_disk() {
    let repository = repository();
    write(&repository, "a.txt", "changed\n");
    let _staged = repository.git(&["add", "a.txt"]);
    let out = on_file(&repository, "changes.unstage", "a.txt");
    assert!(out.contains("> git restore --staged -- a.txt"), "{out}");
    assert_eq!(status(&repository), [" M a.txt"]);
}

#[test]
fn changes_unstage__a_staged_rename_becomes_a_deleted_and_an_untracked_file() {
    let repository = repository();
    let _moved = repository.git(&["mv", "a.txt", "c.txt"]);
    let out = on_file(&repository, "changes.unstage", "c.txt");
    assert!(out.contains("> git restore --staged -- c.txt a.txt"), "{out}");
    assert_eq!(status(&repository), [" D a.txt", "?? c.txt"]);
}

#[test]
fn changes_unstage__before_the_first_commit_the_file_stays_on_disk_untracked() {
    let repository = Repository::new();
    write(&repository, "a.txt", "a\n");
    let _staged = repository.git(&["add", "a.txt"]);
    let out = on_file(&repository, "changes.unstage", "a.txt");
    assert!(out.contains("> git rm --cached -r -- a.txt"), "{out}");
    assert_eq!(status(&repository), ["?? a.txt"]);
}

#[test]
fn changes_stage_all__with_a_conflict_the_conflict_stays() {
    let mut repository = repository();
    repository.conflict();
    write(&repository, "b.txt", "changed\n");
    write(&repository, "c.txt", "new\n");
    let out = cli_done(&repository, &["--target", "changes:list", "changes.stage-all"]).out;
    assert!(out.contains("> git add -- b.txt c.txt"), "{out}");
    let status = status(&repository);
    assert!(status.contains(&format!("UU {CONFLICT_FILE}")), "{status:?}");
    assert!(status.contains(&String::from("A  c.txt")), "{status:?}");
}

#[test]
fn changes_unstage_all__resets_every_box_and_keeps_a_conflict() {
    let repository = repository();
    write(&repository, "a.txt", "changed\n");
    write(&repository, "c.txt", "new\n");
    let _staged = repository.git(&["add", "--all"]);
    let out = cli_done(&repository, &["--target", "changes:list", "changes.unstage-all"]).out;
    assert!(out.contains("> git reset\n"), "{out}");
    assert_eq!(status(&repository), [" M a.txt", "?? c.txt"]);
    let mut conflicted = self::repository();
    conflicted.conflict();
    write(&conflicted, "b.txt", "changed\n");
    let _staged = conflicted.git(&["add", "b.txt"]);
    let out = cli_done(&conflicted, &["--target", "changes:list", "changes.unstage-all"]).out;
    assert!(out.contains("> git restore --staged -- b.txt"), "{out}");
    assert!(status(&conflicted).contains(&format!("UU {CONFLICT_FILE}")));
}

#[test]
fn changes_unstage_all__before_the_first_commit_every_file_stays_on_disk() {
    let repository = Repository::new();
    write(&repository, "a.txt", "a\n");
    write(&repository, "b.txt", "b\n");
    let _staged = repository.git(&["add", "--all"]);
    let out = cli_done(&repository, &["changes.unstage-all"]).out;
    assert!(out.contains("> git rm --cached -r -- ."), "{out}");
    assert_eq!(status(&repository), ["?? a.txt", "?? b.txt"]);
}

#[test]
fn changes_discard__brings_the_file_back_to_head() {
    let repository = repository();
    write(&repository, "a.txt", "changed\n");
    let _out = on_file(&repository, "changes.discard", "a.txt");
    assert_eq!(status(&repository), Vec::<String>::new());
    write(&repository, "new.txt", "new\n");
    let out = on_file(&repository, "changes.discard", "new.txt");
    assert!(out.contains("> git clean -f -- new.txt"), "{out}");
    assert!(!repository.path().join("new.txt").exists());
}

#[test]
fn changes_discard__a_file_partly_staged_and_a_rename_go_back_to_head() {
    let repository = repository();
    write(&repository, "a.txt", "staged\n");
    let _staged = repository.git(&["add", "a.txt"]);
    write(&repository, "a.txt", "and more\n");
    let _out = on_file(&repository, "changes.discard", "a.txt");
    assert_eq!(status(&repository), Vec::<String>::new());
    let _moved = repository.git(&["mv", "b.txt", "c.txt"]);
    let out = on_file(&repository, "changes.discard", "c.txt");
    assert!(
        out.contains("> git restore --source=HEAD --staged --worktree -- c.txt b.txt"),
        "{out}"
    );
    assert_eq!(status(&repository), Vec::<String>::new());
    assert!(repository.path().join("b.txt").exists());
}

#[test]
fn changes_discard_all__from_the_header_leaves_a_conflict_alone() {
    let mut repository = repository();
    repository.conflict();
    write(&repository, "a.txt", "changed\n");
    let _staged = repository.git(&["add", "a.txt"]);
    write(&repository, "new.txt", "new\n");
    let out = cli_done(
        &repository,
        &["--yes", "--target", "changes:list", "changes.discard-all"],
    )
    .out;
    assert!(!out.contains(CONFLICT_FILE), "{out}");
    assert_eq!(status(&repository), [format!("UU {CONFLICT_FILE}")]);
}

#[test]
fn changes_discard_all__from_the_row_leaves_nothing_to_commit() {
    let repository = repository();
    write(&repository, "a.txt", "changed\n");
    write(&repository, "b.txt", "staged\n");
    let _staged = repository.git(&["add", "b.txt"]);
    std::fs::create_dir(repository.path().join("tmp")).unwrap();
    write(&repository, "tmp/x.txt", "x\n");
    let out = cli_done(&repository, &["--yes", "changes.discard-all"]).out;
    let restore = out.find("> git restore --source=HEAD --staged --worktree -- .");
    let clean = out.find("> git clean -f -d -- .");
    assert!(restore.is_some() && restore < clean, "{out}");
    assert_eq!(status(&repository), Vec::<String>::new());
    assert!(!repository.path().join("tmp").exists());
}
