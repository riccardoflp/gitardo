//! The parsers of the details of a commit, of its lines and of a comparison on what git printed in awkward
//! repositories, kept in `tests/golden/<read>`: `<case>.out` is what one command printed, `<case>.expected` what its
//! parser makes of it.
//!
//! With `GITARDO_GOLDEN_RECORD=1`, each test makes its repository, runs the command with the arguments of the read and
//! records both files again; otherwise the parsers read the files of the repository, without git.

use std::ffi::OsString;
use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;

use git::read::changed_files::parse_name_status;
use git::read::commit_page::{PatchRequest, parse_patch, patch_arguments};
use git::read::compare::{
    CompareRequest, Revisions, cherry_arguments, commits_arguments, compare_files_arguments, counts_arguments,
    merge_tree_arguments, parse_cherry_marks, parse_compared_commits, parse_counts, parse_merge_tree,
};
use git::read::details::{commit_arguments, files_arguments, parse_commit};
use git::{GitError, Output};
use model::CommitId;
use model::compare::{CompareMode, Side};
use testkit::{Repository, assert_golden};

use crate::awkward::{awkward_repository, diverged_repository, merge_repository};

/// The file of a case, with its extension.
fn golden_file(case: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{case}.{extension}"))
}

/// When recording, makes the repository of a case and the arguments of its read, runs them there with the
/// configuration every read gets, and keeps what git printed in `<case>.out`, with the folder of the test written
/// `<folder>` so that the file is the same on every machine. Git may end with a code other than 0, as `merge-tree` does
/// to say there are conflicts.
fn record(case: &str, make: impl FnOnce() -> (Repository, Vec<OsString>)) {
    if std::env::var_os("GITARDO_GOLDEN_RECORD").is_none_or(|value| value != "1") {
        return;
    }
    let (repository, arguments) = make();
    let output = repository
        .command("git")
        .args(["-c", "core.quotepath=false"])
        .args(&arguments)
        .output()
        .unwrap();
    let folder = repository.path().parent().unwrap().to_string_lossy().replace('\\', "/");
    let printed = String::from_utf8(output.stdout).unwrap();
    assert_golden(golden_file(case, "out"), printed.replace(&folder, "<folder>"));
}

/// What `<case>.out` holds.
fn recorded(case: &str) -> Vec<u8> {
    fs::read(golden_file(case, "out")).unwrap()
}

/// Parses `<case>.out` and compares what the parser made of it with `<case>.expected`.
fn check<Parsed: Debug>(case: &str, parse: impl Fn(&[u8]) -> Result<Parsed, GitError>) {
    let parsed = parse(&recorded(case)).unwrap();
    assert_golden(golden_file(case, "expected"), format!("{parsed:#?}\n"));
}

/// A commit of the throwaway repositories, by its hash.
fn id(hash: &str) -> CommitId {
    CommitId::parse(hash).unwrap()
}

/// The comparison of `main` with `feature` in [`diverged_repository`], since they diverged.
fn diverged_comparison(repository: &Repository) -> CompareRequest {
    let commit = |name: &str| id(repository.git(&["rev-parse", name]).trim());
    CompareRequest {
        first: Side::Commit(commit("main")),
        second: Side::Commit(commit("feature")),
        mode: CompareMode::SinceDiverged,
        path: None,
    }
}

#[test]
fn details_read_commit__golden_header_of_a_message_with_accents_quotes_a_tab_and_a_trailer() {
    record("details/commit-header", || {
        let (repository, _, second) = awkward_repository();
        (repository, commit_arguments(&id(&second)))
    });
    check("details/commit-header", parse_commit);
}

#[test]
fn details_read_commit__golden_files_changed_in_every_awkward_way() {
    record("details/files-awkward", || {
        let (repository, first, second) = awkward_repository();
        (repository, files_arguments(&id(&second), &[id(&first)]))
    });
    check("details/files-awkward", parse_name_status);
}

#[test]
fn details_read_commit__golden_files_of_the_root_commit() {
    record("details/files-root", || {
        let (repository, first, _) = awkward_repository();
        (repository, files_arguments(&id(&first), &[]))
    });
    check("details/files-root", parse_name_status);
}

#[test]
fn details_read_commit__golden_files_of_a_merge_against_its_first_parent() {
    record("details/files-merge", || {
        let (repository, merge, before) = merge_repository();
        (repository, files_arguments(&id(&merge), &[id(&before)]))
    });
    check("details/files-merge", parse_name_status);
}

/// The lines of a commit, matched to the files its details list.
fn check_patch(case: &str, files_case: &str) {
    let files = parse_name_status(&recorded(files_case)).unwrap();
    check(case, |output| Ok::<_, GitError>(parse_patch(output, &files)));
}

#[test]
fn commit_page_read_patch__golden_lines_of_every_awkward_change() {
    record("commit-page/patch-awkward", || {
        let (repository, first, second) = awkward_repository();
        let request = PatchRequest {
            commit: id(&second),
            first_parent: Some(id(&first)),
            files: Vec::new(),
        };
        (repository, patch_arguments(&request))
    });
    check_patch("commit-page/patch-awkward", "details/files-awkward");
}

#[test]
fn commit_page_read_patch__golden_lines_of_the_root_commit() {
    record("commit-page/patch-root", || {
        let (repository, first, _) = awkward_repository();
        let request = PatchRequest {
            commit: id(&first),
            first_parent: None,
            files: Vec::new(),
        };
        (repository, patch_arguments(&request))
    });
    check_patch("commit-page/patch-root", "details/files-root");
}

#[test]
fn commit_page_read_patch__golden_lines_of_a_merge_against_its_first_parent() {
    record("commit-page/patch-merge", || {
        let (repository, merge, before) = merge_repository();
        let request = PatchRequest {
            commit: id(&merge),
            first_parent: Some(id(&before)),
            files: Vec::new(),
        };
        (repository, patch_arguments(&request))
    });
    check_patch("commit-page/patch-merge", "details/files-merge");
}

#[test]
fn compare_read_sides__golden_counts() {
    record("compare/counts", || {
        let (repository, _) = diverged_repository();
        let arguments = counts_arguments(&Revisions::of(&diverged_comparison(&repository)));
        (repository, arguments)
    });
    check("compare/counts", parse_counts);
}

#[test]
fn compare_read_sides__golden_commits_only_in_each_side() {
    record("compare/commits", || {
        let (repository, _) = diverged_repository();
        let arguments = commits_arguments(&Revisions::of(&diverged_comparison(&repository)));
        (repository, arguments)
    });
    check("compare/commits", parse_compared_commits);
}

#[test]
fn compare_read_sides__golden_commits_marked_as_cherry_picked() {
    record("compare/cherry", || {
        let (repository, _) = diverged_repository();
        let arguments = cherry_arguments(&Revisions::of(&diverged_comparison(&repository)));
        (repository, arguments)
    });
    check("compare/cherry", parse_cherry_marks);
}

#[test]
fn compare_read_sides__golden_files_since_they_diverged() {
    record("compare/files-since", || {
        let (repository, base) = diverged_repository();
        let arguments = compare_files_arguments(&diverged_comparison(&repository), Some(&id(&base)));
        (repository, arguments)
    });
    check("compare/files-since", parse_name_status);
}

#[test]
fn compare_read_sides__golden_files_that_would_conflict() {
    record("compare/merge-tree", || {
        let (repository, _) = diverged_repository();
        let arguments = merge_tree_arguments(&Revisions::of(&diverged_comparison(&repository)));
        (repository, arguments)
    });
    check("compare/merge-tree", |stdout| {
        let output = Output {
            code: Some(1),
            stdout: stdout.to_vec(),
            stderr: Vec::new(),
        };
        Ok::<_, GitError>(parse_merge_tree(&output))
    });
}
