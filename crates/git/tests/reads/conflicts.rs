//! `conflicts.read-files`, `merge-view.read-sides` and `merge.read-predicted-conflicts` against throwaway repositories.

use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;

use git::Git;
use git::read::conflicts::{ConflictFilesRequest, conflict_list_arguments, parse_stages, read_files, stages_arguments};
use git::read::merge::{PredictedConflictsRequest, read_predicted_conflicts};
use git::read::merge_view::{MergeSidesRequest, read_sides};
use model::state::Operation;
use testkit::{CONFLICT_FILE, Repository};

use crate::golden::{Case, check, record};
use crate::{run, write};

/// A merge of `feature` into `main` stopped on a file changed on both sides, `both.txt`, and a file deleted on
/// `feature` and changed on `main`, `deleted on feature.txt`.
fn stopped_merge() -> Repository {
    let mut repository = Repository::new();
    let base = [("both.txt", "base\n"), ("deleted on feature.txt", "base\n")];
    let _ = repository.commit("Add the files", &base);
    repository.branch("feature");
    let ours = [("both.txt", "main\n"), ("deleted on feature.txt", "main\n")];
    let _ = repository.commit("Change on main", &ours);
    repository.checkout("feature");
    let _ = repository.git(&["rm", "--quiet", "--", "deleted on feature.txt"]);
    let _ = repository.commit("Change on feature", &[("both.txt", "feature\n")]);
    repository.checkout("main");
    assert!(!run(&repository, &["merge", "--no-edit", "feature"]).status.success());
    repository
}

/// Which stages a file in conflict has, as numbers.
fn stages(file: &git::read::conflicts::ConflictFile) -> Vec<u8> {
    [
        (1, file.base.is_some()),
        (2, file.current.is_some()),
        (3, file.incoming.is_some()),
    ]
    .into_iter()
    .filter_map(|(stage, has)| has.then_some(stage))
    .collect()
}

#[test]
fn conflicts_read_files__a_file_changed_on_both_sides_is_listed_once_with_three_stages() {
    let repository = stopped_merge();
    let read = read_files(&Git::new(repository.path()), &ConflictFilesRequest).unwrap();
    let both: Vec<_> = read.files.iter().filter(|file| file.path == "both.txt").collect();
    assert_eq!(both.len(), 1);
    assert_eq!(stages(both[0]), [1, 2, 3]);
}

#[test]
fn conflicts_read_files__a_file_deleted_on_feature_and_changed_on_main_has_stages_1_and_2() {
    let repository = stopped_merge();
    let read = read_files(&Git::new(repository.path()), &ConflictFilesRequest).unwrap();
    let deleted = read
        .files
        .iter()
        .find(|file| file.path == "deleted on feature.txt")
        .unwrap();
    assert_eq!(stages(deleted), [1, 2]);
}

#[test]
fn conflicts_read_files__a_name_that_is_a_pattern_is_read_as_that_file() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the pages", &[("pages/[id].tsx", "base\n"), ("pages/i.tsx", "1\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change on main", &[("pages/[id].tsx", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Change on feature", &[("pages/[id].tsx", "feature\n")]);
    repository.checkout("main");
    assert!(!run(&repository, &["merge", "--no-edit", "feature"]).status.success());
    let read = read_files(&Git::new(repository.path()), &ConflictFilesRequest).unwrap();
    let paths: Vec<&str> = read.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["pages/[id].tsx"]);
    assert_eq!(
        stages_arguments(&[String::from("pages/[id].tsx")]).last().unwrap(),
        ":(literal)pages/[id].tsx"
    );
}

/// A repository whose index holds three stages of a path with a tab and of a path with a line break, written straight
/// into the index: no system can hold such names on Windows.
fn awkward_conflicts() -> Repository {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    let blob = repository.git(&["hash-object", "-w", "--", "parser.rs"]);
    let blob = blob.trim_end();
    let mut entries = Vec::new();
    for path in ["with\ttab.txt", "with\nline break.txt"] {
        for stage in 1..=3 {
            entries.push(format!("100644 {blob} {stage}\t{path}\0"));
        }
    }
    let mut child = repository
        .command("git")
        .args(["-c", "core.protectNTFS=false", "update-index", "-z", "--index-info"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(entries.concat().as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success());
    repository
}

#[test]
fn conflicts_read_files__golden_paths_with_a_tab_and_a_line_break() {
    let list = Case {
        read: "conflict-files",
        name: "list",
    };
    record(list, awkward_conflicts, &conflict_list_arguments());
    check(list, git::read::conflicts::parse_conflict_list);
    let paths = [String::from("with\ttab.txt"), String::from("with\nline break.txt")];
    let stages = Case {
        read: "conflict-files",
        name: "stages",
    };
    record(stages, awkward_conflicts, &stages_arguments(&paths));
    check(stages, |output| parse_stages(output).unwrap());
}

#[test]
fn merge_view_read_sides__a_file_added_on_both_sides_has_an_empty_base() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the parser", &[("parser.rs", "1\n")]);
    repository.branch("feature");
    let _ = repository.commit("Add the notes on main", &[("notes.txt", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Add the notes on feature", &[("notes.txt", "feature\n")]);
    repository.checkout("main");
    assert!(!run(&repository, &["merge", "--no-edit", "feature"]).status.success());
    let request = MergeSidesRequest {
        file: String::from("notes.txt"),
        unsaved: None,
    };
    let sides = read_sides(&Git::new(repository.path()), &request).unwrap();
    assert_eq!(sides.base, b"");
    assert_eq!(sides.current, b"main\n");
    assert_eq!(sides.incoming, b"feature\n");
    assert!(String::from_utf8(sides.result).unwrap().contains("<<<<<<<"));
    assert_eq!(sides.operation, Some(Operation::Merge));
}

#[test]
fn merge_view_read_sides__unsaved_edits_of_the_file_are_the_result() {
    let mut repository = Repository::new();
    repository.conflict();
    let request = MergeSidesRequest {
        file: String::from(CONFLICT_FILE),
        unsaved: Some(b"ours and theirs\n".to_vec()),
    };
    let sides = read_sides(&Git::new(repository.path()), &request).unwrap();
    assert_eq!(sides.result, b"ours and theirs\n");
    assert_eq!(sides.base, b"base\n");
    write(&repository, CONFLICT_FILE, "on disk\n");
    let from_disk = MergeSidesRequest {
        unsaved: None,
        ..request
    };
    assert_eq!(
        read_sides(&Git::new(repository.path()), &from_disk).unwrap().result,
        b"on disk\n"
    );
}

#[test]
fn merge_read_predicted_conflicts__git_tells_the_files_a_merge_would_stop_on() {
    let mut repository = Repository::new();
    let _ = repository.commit("Add the files", &[("a.txt", "base\n"), ("b.txt", "base\n")]);
    repository.branch("feature");
    let _ = repository.commit("Change on main", &[("a.txt", "main\n")]);
    repository.checkout("feature");
    let _ = repository.commit("Change on feature", &[("a.txt", "feature\n"), ("b.txt", "feature\n")]);
    repository.checkout("main");
    let request = |theirs: &str| PredictedConflictsRequest {
        ours: None,
        theirs: String::from(theirs),
        base: None,
    };
    let git = Git::new(repository.path());
    let read = read_predicted_conflicts(&git, &request("feature")).unwrap();
    assert_eq!(read, Some(vec![PathBuf::from("a.txt")]));
    let _ = repository.git(&["checkout", "--quiet", "--orphan", "unrelated"]);
    let _ = repository.commit("Start again", &[("c.txt", "1\n")]);
    assert_eq!(read_predicted_conflicts(&git, &request("main")).unwrap(), None);
    assert_eq!(repository.git(&["status", "--porcelain"]), "");
}
