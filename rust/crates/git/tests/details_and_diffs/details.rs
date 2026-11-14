//! `details.read-commit` and `commit-page.read-patch` against throwaway repositories.

use git::Git;
use git::read::commit_page::{PatchRequest, read_patch};
use git::read::details::{CommitRequest, read_commit};
use model::CommitId;
use model::commit_page::{FilePatch, LineKind, PatchLines};
use model::details::{CommitDetail, UNCOMMITTED_SUBJECT};
use model::state::{ChangeKind, FileChange};
use testkit::Repository;

use crate::awkward::{awkward_repository, git_in, merge_repository, stage, write};

/// A commit of the throwaway repositories, by its hash.
fn id(hash: &str) -> CommitId {
    CommitId::parse(hash).unwrap()
}

/// The details of a commit, which must be there.
fn detail(git: &Git, hash: &str) -> CommitDetail {
    read_commit(git, &CommitRequest::Commit(id(hash))).unwrap().unwrap()
}

/// The status letter and the names of each file, as `git diff --name-status` shows them.
fn listed(files: &[FileChange]) -> Vec<String> {
    files
        .iter()
        .map(|file| match &file.original_path {
            Some(original) => format!("{:?} {original} -> {}", file.kind, file.path),
            None => format!("{:?} {}", file.kind, file.path),
        })
        .collect()
}

/// The lines of the files of a commit, which must match them.
fn lines(git: &Git, detail: &CommitDetail) -> Vec<FilePatch> {
    let request = PatchRequest {
        commit: detail.id.clone().unwrap(),
        first_parent: detail.first_parent().cloned(),
        files: detail.files.clone(),
    };
    match read_patch(git, &request).unwrap() {
        PatchLines::Files(files) => files,
        PatchLines::Unmatched => panic!("the lines did not match the files"),
    }
}

#[test]
fn details_read_commit__the_files_of_a_commit_are_those_against_its_parent_renames_with_both_names() {
    let mut repository = Repository::new();
    let first = repository.commit(
        "Add the files",
        &[("old name.txt", "one\ntwo\nthree\n"), ("a.txt", "1\n")],
    );
    let _ = repository.git(&["mv", "old name.txt", "new name.txt"]);
    let second = repository.commit("Rename and change", &[("a.txt", "2\n")]);
    let detail = detail(&git_in(&repository), &second);
    assert_eq!(detail.parents, [id(&first)]);
    assert_eq!(
        listed(&detail.files),
        ["Modified a.txt", "Renamed old name.txt -> new name.txt"]
    );
    assert_eq!(detail.message, "Rename and change");
    let author = detail.author.unwrap();
    assert_eq!(
        (author.name.as_str(), author.email.as_str()),
        ("Test Author", "author@example.com")
    );
}

#[test]
fn details_read_commit__the_files_of_a_merge_are_those_it_changed_against_its_first_parent() {
    let (repository, merge, before) = merge_repository();
    let detail = detail(&git_in(&repository), &merge);
    assert!(detail.is_merge());
    assert_eq!(detail.first_parent(), Some(&id(&before)));
    assert_eq!(listed(&detail.files), ["Added feature.txt", "Modified notes.txt"]);
}

#[test]
fn details_read_commit__the_first_commit_lists_every_file_it_added() {
    let (repository, first, _) = awkward_repository();
    let detail = detail(&git_in(&repository), &first);
    assert_eq!(detail.parents, Vec::new());
    assert_eq!(detail.files.len(), 9);
    assert!(detail.files.iter().all(|file| file.kind == ChangeKind::Added));
}

#[test]
fn details_read_commit__a_stash_with_untracked_files_returns_them_apart() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", b"2\n");
    write(&repository, "new file.txt", b"new\n");
    let _ = repository.git(&["stash", "push", "--quiet", "--include-untracked"]);
    let stash = repository.git(&["rev-parse", "stash@{0}"]);
    let detail = detail(&git_in(&repository), stash.trim());
    assert_eq!(listed(&detail.files), ["Modified parser.rs"]);
    assert_eq!(listed(&detail.untracked_files), ["Added new file.txt"]);
}

#[test]
fn details_read_commit__a_stash_without_untracked_files_or_with_an_empty_untracked_commit_returns_none() {
    let mut repository = Repository::new();
    let first = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    write(&repository, "parser.rs", b"2\n");
    let _ = repository.git(&["stash", "push", "--quiet"]);
    let stash = repository.git(&["rev-parse", "stash@{0}"]).trim().to_owned();
    let git = git_in(&repository);
    assert_eq!(detail(&git, &stash).untracked_files, Vec::new());
    // A stash whose third parent holds the untracked files but has none, made by hand as git would make it.
    let empty_tree = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
    let untracked = repository.git(&["commit-tree", empty_tree, "-m", "untracked files on main: 1234567 Add"]);
    let tree = repository.git(&["rev-parse", &format!("{stash}^{{tree}}")]);
    let index = repository.git(&["rev-parse", &format!("{stash}^2")]);
    let made = repository.git(&[
        "commit-tree",
        tree.trim(),
        "-p",
        &first,
        "-p",
        index.trim(),
        "-p",
        untracked.trim(),
        "-m",
        "WIP on main: 1234567 Add",
    ]);
    let made = detail(&git, made.trim());
    assert_eq!(made.parents.len(), 3);
    assert_eq!(made.untracked_files, Vec::new());
}

#[test]
fn details_read_commit__a_hash_that_is_not_a_commit_returns_no_detail() {
    let (repository, first, _) = awkward_repository();
    let git = git_in(&repository);
    let missing = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(read_commit(&git, &CommitRequest::Commit(id(missing))).unwrap(), None);
    let tree = repository.git(&["rev-parse", &format!("{first}^{{tree}}")]);
    assert_eq!(
        read_commit(&git, &CommitRequest::Commit(id(tree.trim()))).unwrap(),
        None
    );
}

#[test]
fn details_read_commit__the_uncommitted_changes_have_their_subject_no_author_and_every_file() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the files",
        &[("staged.txt", "1\n"), ("changed.txt", "1\n"), ("both.txt", "1\n")],
    );
    write(&repository, "staged.txt", b"2\n");
    write(&repository, "both.txt", b"2\n");
    let _ = repository.git(&["add", "--", "staged.txt", "both.txt"]);
    write(&repository, "both.txt", b"3\n");
    write(&repository, "changed.txt", b"2\n");
    write(&repository, "new.txt", b"new\n");
    let detail = read_commit(&git_in(&repository), &CommitRequest::UncommittedChanges)
        .unwrap()
        .unwrap();
    assert_eq!(detail.message, UNCOMMITTED_SUBJECT);
    assert_eq!((detail.id, detail.author, detail.committer), (None, None, None));
    assert_eq!(
        listed(&detail.files),
        [
            "Modified both.txt",
            "Modified changed.txt",
            "Added new.txt",
            "Modified staged.txt"
        ]
    );
}

#[test]
fn commit_page_read_patch__the_files_come_back_in_the_order_and_number_of_the_details() {
    let (repository, first, second) = awkward_repository();
    let git = git_in(&repository);
    for hash in [&first, &second] {
        let detail = detail(&git, hash);
        let files = lines(&git, &detail);
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        let listed: Vec<&str> = detail.files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, listed);
    }
}

#[test]
fn commit_page_read_patch__a_file_whose_type_changed_is_one_file_of_its_two_parts() {
    let (repository, _, second) = awkward_repository();
    let git = git_in(&repository);
    let detail = detail(&git, &second);
    let files = lines(&git, &detail);
    let link: Vec<&FilePatch> = files.iter().filter(|file| file.path == "link").collect();
    assert_eq!(link.len(), 1);
    assert_eq!(link[0].kind, ChangeKind::TypeChanged);
    assert_eq!(link[0].hunks.len(), 2);
    assert_eq!((link[0].added, link[0].removed), (1, 1));
}

#[test]
fn commit_page_read_patch__files_that_do_not_match_the_output_are_not_shown() {
    let (repository, _, second) = awkward_repository();
    let git = git_in(&repository);
    let mut detail = detail(&git, &second);
    let _ = detail.files.pop();
    let request = PatchRequest {
        commit: detail.id.clone().unwrap(),
        first_parent: detail.first_parent().cloned(),
        files: detail.files.clone(),
    };
    assert_eq!(read_patch(&git, &request).unwrap(), PatchLines::Unmatched);
}

#[test]
fn commit_page_read_patch__no_external_diff_or_text_conversion_of_the_configuration_is_used() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the files",
        &[("a.txt", "abc\n"), (".gitattributes", "*.txt diff=upper\n")],
    );
    let _ = repository.git(&["config", "diff.upper.textconv", "sed s/b/B/"]);
    let _ = repository.git(&["config", "diff.external", "echo EXTERNAL"]);
    let _ = repository.git(&["config", "diff.noprefix", "true"]);
    let second = repository.commit("Change a", &[("a.txt", "abd\n")]);
    let git = git_in(&repository);
    let files = lines(&git, &detail(&git, &second));
    let texts: Vec<(LineKind, &str)> = files[0].hunks[0]
        .lines
        .iter()
        .map(|line| (line.kind, line.text.as_str()))
        .collect();
    assert_eq!(texts, [(LineKind::Removed, "abc"), (LineKind::Added, "abd")]);
}

#[test]
fn commit_page_read_patch__crlf_lines_and_a_missing_newline_are_kept_as_they_are() {
    let mut repository = Repository::new();
    let _ = repository.commit(
        "Add the files",
        &[("windows.txt", "one\r\ntwo\r\n"), ("end.txt", "a\nb")],
    );
    stage(&repository, "100644", "windows.txt", b"one\r\nthree\r\n");
    stage(&repository, "100644", "end.txt", b"a\nc");
    let second = repository.commit("Change them", &[]);
    let git = git_in(&repository);
    let files = lines(&git, &detail(&git, &second));
    let windows = files.iter().find(|file| file.path == "windows.txt").unwrap();
    assert_eq!(windows.hunks[0].lines[2].text, "three\r");
    let end = files.iter().find(|file| file.path == "end.txt").unwrap();
    let kinds: Vec<LineKind> = end.hunks[0].lines.iter().map(|line| line.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::Context,
            LineKind::Removed,
            LineKind::NoNewline,
            LineKind::Added,
            LineKind::NoNewline
        ]
    );
}
