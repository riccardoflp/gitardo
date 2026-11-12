//! The reads of whole files against throwaway repositories: `diff.read-sides`, `file-view.read-revision` and
//! `stash.read-files`.

use git::GitError;
use git::read::diff::{DiffRequest, FileVersion, VersionRequest, read_sides};
use git::read::file_view::{RevisionFileRequest, read_revision};
use git::read::stash::{StashRef, read_files};
use model::CommitId;
use model::diff::DiffSides;
use testkit::Repository;

use crate::awkward::{git_in, stage, write};

/// A commit of the throwaway repositories, by its hash.
fn id(hash: &str) -> CommitId {
    CommitId::parse(hash).unwrap()
}

/// A version of a file, at a path.
fn version(version: FileVersion, path: &str) -> VersionRequest {
    VersionRequest {
        version,
        path: path.to_owned(),
    }
}

/// The two sides of a diff.
fn sides(repository: &Repository, left: VersionRequest, right: VersionRequest) -> DiffSides {
    read_sides(&git_in(repository), &DiffRequest { left, right }).unwrap()
}

#[test]
fn diff_read_sides__unsaved_edits_of_a_file_page_are_the_working_tree_side() {
    let mut repository = Repository::new();
    let head = repository.commit("Add the app", &[("app.rs", "fn main() {}\n")]);
    write(&repository, "app.rs", b"fn main() { saved }\n");
    let unsaved = FileVersion::WorkingTree {
        unsaved: Some(b"fn main() { not saved }\n".to_vec()),
    };
    let read = sides(
        &repository,
        version(FileVersion::Commit(id(&head)), "app.rs"),
        version(unsaved, "app.rs"),
    );
    assert_eq!(read.left.as_deref(), Some(&b"fn main() {}\n"[..]));
    assert_eq!(read.right.as_deref(), Some(&b"fn main() { not saved }\n"[..]));
    let saved = FileVersion::WorkingTree { unsaved: None };
    let read = sides(
        &repository,
        version(FileVersion::Index, "app.rs"),
        version(saved, "app.rs"),
    );
    assert_eq!(read.right.as_deref(), Some(&b"fn main() { saved }\n"[..]));
}

#[test]
fn diff_read_sides__a_renamed_file_is_read_with_its_old_path_in_the_parent_and_its_new_path_in_the_commit() {
    let mut repository = Repository::new();
    let parent = repository.commit("Add", &[("old name.txt", "one\ntwo\nthree\n")]);
    let _ = repository.git(&["mv", "old name.txt", "new name.txt"]);
    let commit = repository.commit("Rename", &[("new name.txt", "one\ntwo\nthree\nfour\n")]);
    let read = sides(
        &repository,
        version(FileVersion::Commit(id(&parent)), "old name.txt"),
        version(FileVersion::Commit(id(&commit)), "new name.txt"),
    );
    assert_eq!(read.left.as_deref(), Some(&b"one\ntwo\nthree\n"[..]));
    assert_eq!(read.right.as_deref(), Some(&b"one\ntwo\nthree\nfour\n"[..]));
}

#[test]
fn diff_read_sides__a_path_with_a_space_and_one_starting_with_a_colon_are_read() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("with space.txt", "space\n")]);
    stage(&repository, "100644", ":colon.txt", b"colon\n");
    let commit = repository.commit("Add a colon", &[]);
    let read = sides(
        &repository,
        version(FileVersion::Commit(id(&commit)), ":colon.txt"),
        version(FileVersion::Index, ":colon.txt"),
    );
    assert_eq!(read.left.as_deref(), Some(&b"colon\n"[..]));
    assert_eq!(read.right.as_deref(), Some(&b"colon\n"[..]));
    let read = sides(
        &repository,
        version(FileVersion::Commit(id(&commit)), "with space.txt"),
        version(FileVersion::WorkingTree { unsaved: None }, "with space.txt"),
    );
    assert_eq!(read.left, read.right);
    assert_eq!(read.left.as_deref(), Some(&b"space\n"[..]));
}

#[test]
fn diff_read_sides__a_version_that_is_not_there_is_none() {
    let mut repository = Repository::new();
    let head = repository.commit("Add", &[("tracked.txt", "1\n")]);
    write(&repository, "untracked.txt", b"new\n");
    let read = sides(
        &repository,
        version(FileVersion::Commit(id(&head)), "untracked.txt"),
        version(FileVersion::Index, "untracked.txt"),
    );
    assert_eq!(read, DiffSides::default());
    std::fs::remove_file(repository.path().join("tracked.txt")).unwrap();
    let read = sides(
        &repository,
        version(FileVersion::Absent, "tracked.txt"),
        version(FileVersion::WorkingTree { unsaved: None }, "tracked.txt"),
    );
    assert_eq!(read, DiffSides::default());
}

/// A file of a commit.
fn revision_file(repository: &Repository, commit: &str, path: &str) -> Result<Option<Vec<u8>>, GitError> {
    let request = RevisionFileRequest {
        commit: id(commit),
        path: path.to_owned(),
    };
    read_revision(&git_in(repository), &request)
}

#[test]
fn file_view_read_revision__the_text_is_the_blob_byte_for_byte_crlf_included() {
    let mut repository = Repository::new();
    let content = "one\r\ntwo\r\n\u{0}é\n";
    let commit = repository.commit(
        "Add",
        &[("windows.txt", content), (".gitattributes", "*.txt diff=upper\n")],
    );
    let _ = repository.git(&["config", "diff.upper.textconv", "sed s/o/O/"]);
    assert_eq!(
        revision_file(&repository, &commit, "windows.txt").unwrap(),
        Some(content.as_bytes().to_vec())
    );
}

#[test]
fn file_view_read_revision__a_path_the_commit_does_not_have_gives_none() {
    let mut repository = Repository::new();
    let commit = repository.commit("Add", &[("a.txt", "1\n")]);
    assert_eq!(revision_file(&repository, &commit, "missing.txt").unwrap(), None);
    assert_eq!(revision_file(&repository, &commit, "a.txt/inside").unwrap(), None);
}

/// What a stash changes.
fn stash_files(repository: &Repository, name: &str) -> Result<model::stash::StashFiles, GitError> {
    read_files(&git_in(repository), &StashRef { name: name.to_owned() })
}

#[test]
fn stash_read_files__a_stash_of_two_changed_files_and_one_untracked_returns_them_apart() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("a.txt", "1\n"), ("b c.txt", "1\n")]);
    write(&repository, "a.txt", b"2\n");
    write(&repository, "b c.txt", b"2\n");
    write(&repository, "new/[draft].md", b"new\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked"]);
    let files = stash_files(&repository, "stash@{0}").unwrap();
    assert_eq!(files.changed, ["a.txt", "b c.txt"]);
    assert_eq!(files.untracked, ["new/[draft].md"]);
}

#[test]
fn stash_read_files__a_stash_made_without_include_untracked_returns_no_untracked_files() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add", &[("a.txt", "1\n")]);
    write(&repository, "a.txt", b"2\n");
    write(&repository, "left.txt", b"left\n");
    let _ = repository.git(&["stash", "push", "--quiet"]);
    let files = stash_files(&repository, "stash@{0}").unwrap();
    assert_eq!(files.changed, ["a.txt"]);
    assert_eq!(files.untracked, Vec::<String>::new());
}

#[test]
fn stash_read_files__a_name_that_is_not_a_stash_is_refused_before_git_runs() {
    let repository = Repository::new();
    let refused = |name: &str| match read_files(
        &git::Git::new("this folder is not there"),
        &StashRef { name: name.to_owned() },
    ) {
        Err(GitError::Refused { reason }) => reason == format!("\"{name}\" is not a stash."),
        _ => false,
    };
    assert!(refused("main"));
    assert!(refused("stash@{x}"));
    assert!(stash_files(&repository, "stash@{0}").is_err());
}
