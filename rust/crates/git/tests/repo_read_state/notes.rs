//! `note.read` on throwaway repositories: the commits with a note in the reference git uses, read with the state, and
//! the text of one note, read with the details of its commit.

use git::Git;
use git::read::details::{CommitRequest, read_commit};
use git::read::note::{NoteRequest, parse_list, read};
use model::CommitId;
use testkit::Repository;

use super::{read as read_state, write};

/// A repository of one commit, and its full id.
fn one_commit() -> (Repository, CommitId) {
    let mut repository = Repository::new();
    let hash = repository.commit("Add the parser", &[("parser.rs", "fn parse() {}\n")]);
    (repository, CommitId::parse(hash.trim()).unwrap())
}

/// The text of the note of a commit, as the details of the commit read it.
fn note_text(repository: &Repository, commit: &CommitId) -> Option<String> {
    let git = Git::new(repository.path());
    read_commit(&git, &CommitRequest::Commit(commit.clone()))
        .unwrap()
        .unwrap()
        .note
}

#[test]
fn note_read__a_commit_with_a_note_is_listed_and_its_text_is_read_as_written_lines_and_all() {
    let (repository, commit) = one_commit();
    let _added = repository.git(&[
        "notes",
        "add",
        "-m",
        "Reviewed.",
        "-m",
        "Shipped in 2.1.",
        commit.as_str(),
    ]);
    let state = read_state(&repository);
    assert!(state.notes.has_note(&commit), "{:?}", state.notes);
    assert_eq!(state.notes.reference, "refs/notes/commits");
    assert_eq!(
        note_text(&repository, &commit).as_deref(),
        Some("Reviewed.\n\nShipped in 2.1.")
    );
}

#[test]
fn note_read__a_repository_with_no_reference_of_notes_reads_no_note_and_no_error() {
    let (repository, commit) = one_commit();
    let state = read_state(&repository);
    assert!(state.notes.noted.is_empty());
    assert_eq!(state.notes.tip, None);
    assert_eq!(note_text(&repository, &commit), None);
    assert!(parse_list(b"").unwrap().is_empty());
}

#[test]
fn note_read__with_core_notes_ref_set_the_notes_of_that_reference_are_read() {
    let (repository, commit) = one_commit();
    let _review = repository.git(&["notes", "--ref=review", "add", "-m", "Looked at.", commit.as_str()]);
    let _default = repository.git(&["config", "core.notesRef", "refs/notes/review"]);
    let state = read_state(&repository);
    assert_eq!(state.notes.reference, "refs/notes/review");
    assert!(state.notes.has_note(&commit));
    assert_eq!(note_text(&repository, &commit).as_deref(), Some("Looked at."));
}

#[test]
fn note_read__a_read_after_a_change_to_a_file_only_keeps_the_list_without_running_git_notes_list() {
    let (repository, commit) = one_commit();
    let _added = repository.git(&["notes", "add", "-m", "Reviewed.", commit.as_str()]);
    let git = Git::new(repository.path());
    let first = read(&git, &NoteRequest::default()).unwrap();
    assert!(first.has_note(&commit));
    write(&repository, "parser.rs", "fn parse() { todo() }\n");
    // A list the reference never had: kept as it is, it shows that `git notes list` did not run again.
    let mut kept = first.clone();
    kept.noted.clear();
    let again = read(
        &git,
        &NoteRequest {
            before: Some(kept.clone()),
        },
    )
    .unwrap();
    assert_eq!(again, kept);
    let _second = repository.git(&["notes", "append", "-m", "Merged.", commit.as_str()]);
    let moved = read(&git, &NoteRequest { before: Some(kept) }).unwrap();
    assert!(moved.has_note(&commit), "the reference moved: the list is read again");
}
