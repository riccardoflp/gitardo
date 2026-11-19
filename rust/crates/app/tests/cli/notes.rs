//! The notes of commits written, removed, fetched and pushed by `gitardo-cli` on throwaway repositories, the remote a
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

#[test]
fn note_push__it_runs_git_push_origin_refs_notes_commits_and_a_fetch_of_the_notes_then_has_them() {
    let (repository, remote, commit) = pushed();
    let _added = repository.git(&["notes", "add", "-m", "Reviewed.", &commit]);
    let ended = cli_done(&repository, &["--target", "remote:origin", "note.push"]);
    assert!(
        ended.out.contains("> git push origin refs/notes/commits"),
        "{}",
        ended.out
    );
    assert!(remote.git(&["notes", "show", &commit]).contains("Reviewed."));
    // The notes gone from here, as in another clone: fetching them brings them back.
    let _gone = repository.git(&["update-ref", "-d", "refs/notes/commits"]);
    let _fetched = cli_done(&repository, &["--target", "remote:origin", "note.fetch"]);
    assert_eq!(note_of(&repository, &commit), "Reviewed.\n");
}

#[test]
fn note_fetch__from_a_remote_whose_notes_are_ahead_it_fetches_its_reference_into_the_same_one() {
    let (repository, remote, commit) = pushed();
    let _on_remote = remote.git(&["notes", "add", "-m", "Released in 2.1.", &commit]);
    let ended = cli_done(&repository, &["--target", "remote:origin", "note.fetch"]);
    assert!(
        ended
            .out
            .contains("> git fetch origin refs/notes/commits:refs/notes/commits"),
        "{}",
        ended.out
    );
    assert_eq!(note_of(&repository, &commit), "Released in 2.1.\n");
}

#[test]
fn note_fetch__with_a_note_added_on_each_side_the_refusal_is_followed_by_merge_the_notes_which_keeps_both() {
    let (repository, remote, commit) = pushed();
    let _on_remote = remote.git(&["notes", "add", "-m", "Released in 2.1.", &commit]);
    let _here = repository.git(&["notes", "add", "-m", "Reviewed.", &commit]);
    let ended = cli_done(&repository, &["--yes", "--target", "remote:origin", "note.fetch"]);
    assert!(
        ended
            .out
            .contains("> git notes merge --strategy=cat_sort_uniq refs/notes/remotes/origin/commits"),
        "{}",
        ended.out
    );
    let note = note_of(&repository, &commit);
    assert!(
        note.contains("Reviewed.") && note.contains("Released in 2.1."),
        "{note}"
    );
}

#[test]
fn note_fetch__from_a_remote_with_no_notes_it_says_so_and_nothing_changes() {
    let (repository, _remote, _commit) = pushed();
    let ended = cli(&repository, &["--target", "remote:origin", "note.fetch"]);
    assert_eq!(ended.code, Some(1));
    assert!(
        ended.errors.contains("origin has no notes."),
        "{}{}",
        ended.out,
        ended.errors
    );
    assert_eq!(repository.git(&["for-each-ref", "refs/notes"]), "");
}

#[test]
fn note_push__when_the_remote_has_a_note_this_repository_has_not_the_refusal_offers_fetch_notes() {
    let (mut repository, remote, commit) = pushed();
    let _added = repository.git(&["notes", "add", "-m", "Reviewed.", &commit]);
    let _pushed = cli_done(&repository, &["--target", "remote:origin", "note.push"]);
    let first = revision(&repository, "HEAD~1");
    let _on_remote = remote.git(&["notes", "add", "-m", "Released in 2.1.", &first]);
    let _ = repository.commit("Add the lexer", &[("lexer.rs", "fn lex() {}\n")]);
    let _here = repository.git(&["notes", "add", "-m", "Tested.", "HEAD"]);
    let ended = cli(&repository, &["--target", "remote:origin", "note.push"]);
    assert_eq!(ended.code, Some(1), "{}", ended.out);
    assert!(
        ended.out.contains("Next: Fetch notes (note.fetch)"),
        "{}{}",
        ended.out,
        ended.errors
    );
}
