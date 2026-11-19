//! The notes of commits written and removed by `gitardo-cli` on throwaway repositories, the remote a
//! bare repository in the same throwaway folder: never the network.

use testkit::{Remote, Repository};

use crate::{cli, cli_done, revision};

/// A repository of two commits, pushed to its bare remote `origin`, and the full id of its last commit.
fn pushed() -> (Repository, Remote, String) {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the readme", &[("README.md", "Gitardo\n")]);
    let _ = repository.commit("Add the parser", &[("parser.rs", "fn parse() {}\n")]);
    let remote = Remote::new(&repository);
    let commit = revision(&repository, "HEAD");
    (repository, remote, commit)
}

/// The note of a commit as `git notes show` prints it, or nothing.
fn note_of(repository: &Repository, commit: &str) -> String {
    repository
        .command("git")
        .args(["notes", "show", commit])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default()
}

#[test]
fn note_edit__add_note_runs_git_notes_add_with_the_text_on_its_input_and_git_notes_show_prints_it() {
    let (repository, _remote, commit) = pushed();
    let target = format!("commit:{commit}");
    let ended = cli_done(&repository, &["--target", &target, "note.edit", "note=Reviewed."]);
    assert!(
        ended.out.contains(&format!("> git notes add -f -F - {commit}")),
        "{}",
        ended.out
    );
    assert_eq!(note_of(&repository, &commit), "Reviewed.\n");
}

#[test]
fn note_edit__edit_note_proposes_the_note_it_has_and_the_text_changed_replaces_it() {
    let (repository, _remote, commit) = pushed();
    let _added = repository.git(&["notes", "add", "-m", "Reviewed.", &commit]);
    let target = format!("commit:{commit}");
    let asked = cli(&repository, &["--target", &target, "note.edit"]);
    assert_eq!(asked.code, Some(2), "the note is asked, the one it has proposed");
    let _edited = cli_done(
        &repository,
        &["--target", &target, "note.edit", "note=Reviewed and tested."],
    );
    assert_eq!(note_of(&repository, &commit), "Reviewed and tested.\n");
}

#[test]
fn note_remove__on_a_commit_with_a_note_it_runs_git_notes_remove() {
    let (repository, _remote, commit) = pushed();
    let _added = repository.git(&["notes", "add", "-m", "Reviewed.", &commit]);
    let target = format!("commit:{commit}");
    let ended = cli_done(&repository, &["--target", &target, "note.remove"]);
    assert!(
        ended.out.contains(&format!("> git notes remove {commit}")),
        "{}",
        ended.out
    );
    assert_eq!(note_of(&repository, &commit), "");
}
