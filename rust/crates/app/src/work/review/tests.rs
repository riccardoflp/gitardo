#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
#![allow(clippy::unwrap_used, clippy::panic, reason = "a test that cannot go on fails")]

use git::CancelToken;
use model::review::{KeptChanges, KeptRow, SideRow, Which};
use testkit::Repository;

use super::{ReviewReadout, ReviewRequest, diff_of, read_review};

/// A file's path and its texts in the base, on `main` and on `feature`.
pub type Texts<'a> = (&'a str, &'a str, Option<&'a str>, Option<&'a str>);

/// A repository whose merge of `feature` into `main` stopped on conflicts in some files, each given as its path and
/// its text in the base, on `main` and on `feature`; a text of `None` deletes the file on that side.
pub fn merged(files: &[Texts<'_>]) -> Repository {
    let mut repository = Repository::new();
    let base: Vec<(&str, &str)> = files.iter().map(|(path, base, _, _)| (*path, *base)).collect();
    let _base = repository.commit("Add the files", &base);
    repository.branch("feature");
    side(&mut repository, files, |(_, _, main, _)| *main, "Change them on main");
    repository.checkout("feature");
    side(
        &mut repository,
        files,
        |(_, _, _, feature)| *feature,
        "Change them on feature",
    );
    repository.checkout("main");
    let _status = repository
        .command("git")
        .args(["merge", "--no-ff", "--no-edit", "feature"])
        .output()
        .unwrap();
    repository
}

/// Commits the texts of one side, deleting the files it has none of.
fn side<'a>(
    repository: &mut Repository,
    files: &[Texts<'a>],
    text: impl Fn(&Texts<'a>) -> Option<&'a str>,
    message: &str,
) {
    let kept: Vec<(&str, &str)> = files
        .iter()
        .filter_map(|file| text(file).map(|text| (file.0, text)))
        .collect();
    for file in files.iter().filter(|file| text(file).is_none()) {
        let _ = repository.git(&["rm", "--quiet", "--", file.0]);
    }
    if kept.is_empty() {
        let _ = repository.git(&["commit", "--quiet", "-m", message]);
    } else {
        let _ = repository.commit(message, &kept);
    }
}

/// What the review reads of a repository.
fn read(repository: &Repository) -> ReviewReadout {
    read_review(repository.path(), &ReviewRequest::default(), &CancelToken::new()).unwrap()
}

/// The rows of a side as a diff writes them: `- b`, `+ B`, `  c`.
fn written(rows: &[SideRow]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            SideRow::Same { text, .. } => format!("  {text}"),
            SideRow::Removed { text, .. } => format!("- {text}"),
            SideRow::Added { text, .. } => format!("+ {text}"),
        })
        .collect()
}

#[test]
fn review_side_diff__lines_next_to_each_other_show_what_each_side_changed() {
    let repository = merged(&[("a.txt", "a\nb\nc\n", Some("a\nB\nc\n"), Some("a\nb\nC\n"))]);
    let readout = read(&repository);
    let (current, incoming) = readout.side_diffs.first().and_then(|file| file.first()).unwrap();
    assert_eq!(written(&current.rows), ["- b", "+ B", "  c"]);
    assert_eq!(written(&incoming.rows), ["  b", "- c", "+ C"]);
    assert!(current.is_compared);
}

#[test]
fn review_side_diff__the_lines_around_are_numbered_as_in_the_file_on_disk() {
    let base = "1\n2\n3\n4\na\nb\nc\n5\n6\n7\n8\n";
    let main = "1\n2\n3\n4\na\nB\nc\n5\n6\n7\n8\n";
    let feature = "1\n2\n3\n4\na\nb\nC\n5\n6\n7\n8\n";
    let repository = merged(&[("a.txt", base, Some(main), Some(feature))]);
    let readout = read(&repository);
    let file = readout.read.files.first().unwrap();
    let on_disk: Vec<&str> = file.text.lines().collect();
    let (current, _) = readout.side_diffs.first().and_then(|file| file.first()).unwrap();
    for (number, text) in current.before.iter().chain(&current.after) {
        assert_eq!(on_disk.get(number - 1).copied(), Some(text.as_str()), "line {number}");
    }
    assert_eq!(current.before.len(), 3);
    assert_eq!(current.after.len(), 3);
}

#[test]
fn review_side_diff__a_conflict_without_a_stretch_shows_its_lines_as_they_are() {
    let repository = merged(&[("a.txt", "a\nb\nc\n", Some("a\nB1\nc\n"), Some("a\nB2\nc\n"))]);
    let mut readout = read(&repository);
    let file = readout.read.files.first_mut().unwrap();
    let conflict = file.conflicts.first_mut().unwrap();
    conflict.stretch = None;
    let conflict = conflict.clone();
    let diff = diff_of(file, &conflict, Which::Current);
    assert_eq!(written(&diff.rows), ["  B1"]);
    assert!(!diff.is_compared);
}

#[test]
fn reads_a_file_deleted_on_one_side_with_the_changes_of_the_other() {
    let base = "one\ntwo\nthree\n";
    let repository = merged(&[("gone.txt", base, Some("one\nTWO\nthree\n"), None)]);
    let readout = read(&repository);
    assert_eq!(readout.read.files.len(), 0);
    let deleted = readout.read.deleted.first().unwrap();
    assert_eq!(deleted.path, "gone.txt");
    assert_eq!(deleted.deleted_by, Which::Incoming);
    let KeptChanges::Rows(rows) = &deleted.changes else {
        panic!("rows: {:?}", deleted.changes)
    };
    assert!(rows.contains(&KeptRow::Removed(String::from("two"))));
    assert!(rows.contains(&KeptRow::Added(2, String::from("TWO"))));
}
