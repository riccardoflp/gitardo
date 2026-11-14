//! The reads of a comparison against throwaway repositories: `compare.resolve-revision`, `compare.read-sides`,
//! `compare.read-patch` and `compare.read-files`.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::Stdio;

use git::read::compare::{
    ComparePatchRequest, CompareRequest, FilesRequest, PatchForm, RevisionRequest, read_files, read_patch, read_sides,
    resolve_revision,
};
use git::{Git, GitError};
use model::CommitId;
use model::commit_page::PatchLines;
use model::compare::{CommitSide, CompareMode, ComparePatch, CompareView, MergeOutlook, Side};
use testkit::Repository;

use crate::awkward::{diverged_repository, git_in, write};

/// The commit a name stands for in a throwaway repository.
fn commit(repository: &Repository, name: &str) -> CommitId {
    CommitId::parse(repository.git(&["rev-parse", &format!("{name}^{{commit}}")]).trim()).unwrap()
}

/// The comparison of two names of a repository.
fn request(repository: &Repository, first: &str, second: &str, mode: CompareMode) -> CompareRequest {
    CompareRequest {
        first: Side::Commit(commit(repository, first)),
        second: Side::Commit(commit(repository, second)),
        mode,
        path: None,
    }
}

/// The paths of the files of a comparison.
fn paths(view: &CompareView) -> Vec<&str> {
    view.files.iter().map(|file| file.path.as_str()).collect()
}

/// The commit a revision is, in a repository.
fn resolved(git: &Git, text: &str) -> Option<CommitId> {
    resolve_revision(git, &RevisionRequest { text: text.to_owned() }).unwrap()
}

/// A git that cannot run: its folder is not there, so that a read that runs it fails.
fn git_that_cannot_run() -> Git {
    Git::new("this folder is not there")
}

#[test]
fn compare_resolve_revision__ancestors_reflog_dates_and_short_hashes_return_the_full_hash() {
    let mut repository = Repository::new();
    let first = repository.commit("One", &[("a.txt", "1\n")]);
    let _ = repository.commit("Two", &[("a.txt", "2\n")]);
    repository.tag("v1.0");
    let third = repository.commit("Three", &[("a.txt", "3\n")]);
    let fourth = repository.commit("Four", &[("a.txt", "4\n")]);
    let git = git_in(&repository);
    assert_eq!(resolved(&git, "HEAD~3"), Some(CommitId::parse(&first).unwrap()));
    assert_eq!(resolved(&git, "v1.0^"), Some(CommitId::parse(&first).unwrap()));
    assert_eq!(resolved(&git, &third[..7]), Some(CommitId::parse(&third).unwrap()));
    assert_eq!(
        resolved(&git, "main@{yesterday}"),
        Some(CommitId::parse(&fourth).unwrap())
    );
}

#[test]
fn compare_resolve_revision__a_tag_returns_its_commit_also_an_annotated_one() {
    let mut repository = Repository::new();
    let first = repository.commit("One", &[("a.txt", "1\n")]);
    repository.tag("light");
    let _ = repository.git(&["tag", "--annotate", "annotated", "--message", "A release"]);
    let _ = repository.commit("Two", &[("a.txt", "2\n")]);
    let git = git_in(&repository);
    let expected = Some(CommitId::parse(&first).unwrap());
    assert_eq!(resolved(&git, "light"), expected);
    assert_eq!(resolved(&git, "annotated"), expected);
}

#[test]
fn compare_resolve_revision__a_name_git_does_not_know_returns_none_and_no_error() {
    let mut repository = Repository::new();
    let first = repository.commit("One", &[("a.txt", "1\n")]);
    let git = git_in(&repository);
    assert_eq!(resolved(&git, "no-such-branch"), None);
    assert_eq!(resolved(&git, "HEAD~5"), None);
    assert_eq!(resolved(&git, &format!("{first}^{{tree}}")), None);
}

#[test]
fn compare_resolve_revision__an_empty_text_or_one_starting_with_a_dash_returns_none_without_running_git() {
    let git = git_that_cannot_run();
    assert_eq!(resolved(&git, ""), None);
    assert_eq!(resolved(&git, "--all"), None);
    assert!(
        resolve_revision(
            &git,
            &RevisionRequest {
                text: String::from("HEAD")
            }
        )
        .is_err()
    );
}

#[test]
fn compare_read_sides__two_branches_three_and_two_apart_return_3_2_and_their_merge_base() {
    let (repository, base) = diverged_repository();
    let view = read_sides(
        &git_in(&repository),
        &request(&repository, "feature", "main", CompareMode::SinceDiverged),
    )
    .unwrap();
    assert_eq!((view.only_in_first, view.only_in_second), (3, 2));
    assert_eq!(view.merge_base, Some(CommitId::parse(&base).unwrap()));
    let sides: Vec<CommitSide> = view.commits.iter().map(|commit| commit.side).collect();
    assert_eq!(sides.iter().filter(|side| **side == CommitSide::First).count(), 3);
    assert_eq!(sides.len(), 5);
    assert_eq!(view.merge, MergeOutlook::Conflicts(vec![String::from("shared.txt")]));
}

#[test]
fn compare_read_sides__since_they_diverged_the_files_are_those_the_second_changed() {
    let (repository, _) = diverged_repository();
    let view = read_sides(
        &git_in(&repository),
        &request(&repository, "main", "feature", CompareMode::SinceDiverged),
    )
    .unwrap();
    assert_eq!(
        paths(&view),
        ["pages/[id].tsx", "pages/i.tsx", "shared.txt", "src/app.rs"]
    );
}

#[test]
fn compare_read_sides__in_everything_that_differs_the_files_are_those_that_differ() {
    let (repository, _) = diverged_repository();
    let view = read_sides(
        &git_in(&repository),
        &request(&repository, "main", "feature", CompareMode::Everything),
    )
    .unwrap();
    assert_eq!(paths(&view), ["pages/[id].tsx", "pages/i.tsx", "shared.txt"]);
}

#[test]
fn compare_read_sides__with_the_working_tree_second_a_file_changed_and_not_committed_is_listed() {
    let (repository, _) = diverged_repository();
    write(&repository, "src/app.rs", b"changed here\n");
    let comparison = CompareRequest {
        first: Side::Commit(commit(&repository, "main")),
        second: Side::WorkingTree,
        mode: CompareMode::Everything,
        path: None,
    };
    let view = read_sides(&git_in(&repository), &comparison).unwrap();
    assert_eq!(paths(&view), ["src/app.rs"]);
    assert_eq!(view.merge, MergeOutlook::NotAsked);
    let reversed = CompareRequest {
        first: Side::WorkingTree,
        second: comparison.first.clone(),
        ..comparison
    };
    assert_eq!(
        paths(&read_sides(&git_in(&repository), &reversed).unwrap()),
        ["src/app.rs"]
    );
}

#[test]
fn compare_read_sides__a_commit_cherry_picked_to_the_other_side_is_marked_also_in() {
    let (repository, _) = diverged_repository();
    let view = read_sides(
        &git_in(&repository),
        &request(&repository, "main", "feature", CompareMode::SinceDiverged),
    )
    .unwrap();
    let also_in: Vec<&str> = view
        .commits
        .iter()
        .filter(|commit| commit.is_also_in_other_side)
        .map(|commit| commit.subject.as_str())
        .collect();
    assert_eq!(also_in, ["Fix the app: [draft] é", "Fix the app: [draft] é"]);
}

/// Adds so many commits on top of a branch at once, with `git fast-import`, each changing one file and one of them a
/// change the other side has too.
fn add_many_commits(repository: &Repository, branch: &str, count: usize) {
    let mut stream = String::new();
    for number in 0..count {
        let time = 1_800_000_000 + number;
        let _ = writeln!(
            stream,
            "commit refs/heads/{branch}\ncommitter Test Author <author@example.com> {time} +0000\ndata 7\nCommit"
        );
        if number == 0 {
            let _ = writeln!(stream, "from refs/heads/{branch}^0");
        }
        let content = format!("{number}\n");
        let _ = writeln!(stream, "M 100644 inline many.txt\ndata {}\n{content}", content.len());
    }
    let mut child = repository
        .command("git")
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stream.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success());
}

#[test]
fn compare_read_sides__past_2000_commits_none_is_marked_and_the_newest_2000_are_returned() {
    let (repository, _) = diverged_repository();
    add_many_commits(&repository, "feature", 2_000);
    let view = read_sides(
        &git_in(&repository),
        &request(&repository, "main", "feature", CompareMode::SinceDiverged),
    )
    .unwrap();
    assert_eq!((view.only_in_first, view.only_in_second), (2, 2_003));
    assert_eq!(view.commits.len(), 2_000);
    assert!(view.commits.iter().all(|commit| !commit.is_also_in_other_side));
    assert_eq!(view.commits[0].id, commit(&repository, "feature"));
}

#[test]
fn compare_read_sides__narrowed_to_a_file_only_that_file_and_the_commits_that_change_it_are_returned() {
    let (repository, _) = diverged_repository();
    let mut comparison = request(&repository, "main", "feature", CompareMode::SinceDiverged);
    comparison.path = Some(String::from("src/app.rs"));
    let view = read_sides(&git_in(&repository), &comparison).unwrap();
    assert_eq!(paths(&view), ["src/app.rs"]);
    let subjects: Vec<&str> = view.commits.iter().map(|commit| commit.subject.as_str()).collect();
    assert_eq!(subjects, ["Fix the app: [draft] é", "Fix the app: [draft] é"]);
    assert_eq!((view.only_in_first, view.only_in_second), (1, 1));
}

#[test]
fn compare_read_sides__a_file_named_like_a_pattern_narrows_to_that_file_only() {
    let (repository, _) = diverged_repository();
    let mut comparison = request(&repository, "main", "feature", CompareMode::Everything);
    comparison.path = Some(String::from("pages/[id].tsx"));
    let view = read_sides(&git_in(&repository), &comparison).unwrap();
    assert_eq!(paths(&view), ["pages/[id].tsx"]);
}

#[test]
fn compare_read_sides__sides_that_are_not_commits_or_both_the_working_tree_are_refused() {
    let (repository, base) = diverged_repository();
    let git = git_in(&repository);
    let refused = |result: Result<CompareView, GitError>| match result {
        Err(GitError::Refused { reason }) => reason == "These are not commits of this repository.",
        _ => false,
    };
    let both = CompareRequest {
        first: Side::WorkingTree,
        second: Side::WorkingTree,
        mode: CompareMode::Everything,
        path: None,
    };
    assert!(refused(read_sides(&git_that_cannot_run(), &both)));
    let tree = repository.git(&["rev-parse", &format!("{base}^{{tree}}")]);
    let not_commit = CompareRequest {
        first: Side::Commit(CommitId::parse(tree.trim()).unwrap()),
        second: Side::Commit(commit(&repository, "main")),
        mode: CompareMode::Everything,
        path: None,
    };
    assert!(refused(read_sides(&git, &not_commit)));
}

/// The lines of a comparison read as the page reads them, after its sides.
fn compare_lines(git: &Git, comparison: &CompareRequest) -> (CompareView, PatchLines) {
    let view = read_sides(git, comparison).unwrap();
    let patch = ComparePatchRequest {
        comparison: comparison.clone(),
        merge_base: view.merge_base.clone(),
        files: view.files.clone(),
        form: PatchForm::Lines,
    };
    match read_patch(git, &patch).unwrap() {
        ComparePatch::Lines(lines) => (view, lines),
        ComparePatch::Text(_) => panic!("lines were asked for"),
    }
}

#[test]
fn compare_read_patch__the_files_come_back_in_the_order_and_number_of_the_sides() {
    let (repository, _) = diverged_repository();
    write(&repository, "shared.txt", b"working\n");
    let git = git_in(&repository);
    for mode in [CompareMode::SinceDiverged, CompareMode::Everything] {
        let (view, lines) = compare_lines(&git, &request(&repository, "main", "feature", mode));
        let PatchLines::Files(files) = lines else {
            panic!("the lines did not match the files in {mode:?}")
        };
        let patched: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(patched, paths(&view));
    }
    let with_working_tree = CompareRequest {
        first: Side::WorkingTree,
        second: Side::Commit(commit(&repository, "feature")),
        mode: CompareMode::Everything,
        path: None,
    };
    let (view, lines) = compare_lines(&git, &with_working_tree);
    let PatchLines::Files(files) = lines else {
        panic!("the lines did not match the files")
    };
    assert_eq!(files.len(), view.files.len());
}

#[test]
fn compare_read_patch__the_patch_to_copy_includes_binary_files_and_git_apply_takes_it_whole() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("image.bin", "\0\u{1}\u{2}"), ("a.txt", "1\n")]);
    repository.branch("feature");
    repository.checkout("feature");
    let _ = repository.commit("Change them", &[("image.bin", "\0\u{3}\u{4}\u{5}"), ("a.txt", "2\n")]);
    repository.checkout("main");
    let git = git_in(&repository);
    let comparison = request(&repository, "main", "feature", CompareMode::Everything);
    let view = read_sides(&git, &comparison).unwrap();
    let patch = ComparePatchRequest {
        comparison,
        merge_base: view.merge_base,
        files: view.files,
        form: PatchForm::Text,
    };
    let ComparePatch::Text(text) = read_patch(&git, &patch).unwrap() else {
        panic!("a patch was asked for")
    };
    assert!(String::from_utf8_lossy(&text).contains("GIT binary patch"));
    let file = repository.path().parent().unwrap().join("comparison.patch");
    std::fs::write(&file, &text).unwrap();
    let _ = repository.git(&["apply", file.to_str().unwrap()]);
    assert_eq!(
        std::fs::read(repository.path().join("image.bin")).unwrap(),
        b"\0\x03\x04\x05"
    );
    assert_eq!(std::fs::read(repository.path().join("a.txt")).unwrap(), b"2\n");
}

#[test]
fn compare_read_files__the_files_that_differ_between_two_commits_renames_with_both_names() {
    let mut repository = Repository::new();
    let first = repository.commit("Add", &[("old.txt", "one\ntwo\nthree\n"), ("a.txt", "1\n")]);
    let _ = repository.git(&["mv", "old.txt", "new.txt"]);
    let second = repository.commit("Rename", &[("a.txt", "2\n")]);
    let request = FilesRequest {
        from: first,
        to: second,
    };
    let comparison = read_files(&git_in(&repository), &request).unwrap();
    let names: Vec<(Option<&str>, &str)> = comparison
        .files
        .iter()
        .map(|file| (file.original_path.as_deref(), file.path.as_str()))
        .collect();
    assert_eq!(names, [(None, "a.txt"), (Some("old.txt"), "new.txt")]);
}

#[test]
fn compare_read_files__sides_that_are_not_full_hashes_return_nothing_without_running_git() {
    let git = git_that_cannot_run();
    let full = "a".repeat(40);
    for (from, to) in [("main", full.as_str()), (full.as_str(), "3f2a1c9"), ("", "")] {
        let request = FilesRequest {
            from: from.to_owned(),
            to: to.to_owned(),
        };
        assert_eq!(read_files(&git, &request).unwrap().files, Vec::new());
    }
}
