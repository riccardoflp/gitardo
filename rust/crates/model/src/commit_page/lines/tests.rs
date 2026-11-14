#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use std::fmt::Write;

use super::diff_lines;
use crate::commit_page::{DiffOutput, FilePatch, LINE_LIMIT, LineKind, PatchLines};
use crate::state::{ChangeKind, FileChange};

/// A file listed, changed in place.
fn modified(path: &str) -> FileChange {
    FileChange {
        path: path.to_owned(),
        original_path: None,
        kind: ChangeKind::Modified,
    }
}

/// The diff of a file changed in place, with so many hunks of so many added lines each.
fn diff_of(path: &str, hunks: usize, lines: usize) -> String {
    let mut text = format!("diff --git a/{path} b/{path}\nindex 1111111..2222222 100644\n--- a/{path}\n+++ b/{path}\n");
    for hunk in 0..hunks {
        let start = hunk * 100 + 1;
        let _ = writeln!(text, "@@ -{start},0 +{start},{lines} @@");
        for line in 0..lines {
            let _ = writeln!(text, "+line {line}");
        }
    }
    text
}

/// The patches of an output whose files are listed, with a limit.
fn files(listed: &[FileChange], output: &str, limit: usize) -> Vec<FilePatch> {
    let input = DiffOutput {
        files: listed,
        output: output.as_bytes(),
        limit,
    };
    match diff_lines(&input) {
        PatchLines::Files(files) => files,
        PatchLines::Unmatched => panic!("the output did not match the files listed"),
    }
}

#[test]
fn commit_page_diff_lines__a_file_of_3_hunks_with_10_lines_each_counts_33_lines() {
    let found = files(&[modified("a.txt")], &diff_of("a.txt", 3, 10), LINE_LIMIT);
    assert_eq!(found[0].size(), 33);
    assert_eq!(found[0].hunks.len(), 3);
    let exactly = files(&[modified("a.txt")], &diff_of("a.txt", 3, 10), 33);
    assert!(!exactly[0].is_past_limit);
    let under = files(&[modified("a.txt")], &diff_of("a.txt", 3, 10), 32);
    assert!(under[0].is_past_limit);
}

#[test]
fn commit_page_diff_lines__the_counts_of_a_file_left_without_lines_are_still_its_lines() {
    let listed = [modified("big.txt"), modified("middle.txt"), modified("small.txt")];
    let output = [
        diff_of("big.txt", 1, 14_999),
        diff_of("middle.txt", 1, 7_999),
        diff_of("small.txt", 1, 2_999),
    ]
    .concat();
    let found = files(&listed, &output, LINE_LIMIT);
    let shown: Vec<bool> = found.iter().map(|file| !file.hunks.is_empty()).collect();
    assert_eq!(shown, [true, false, true]);
    assert!(found[1].is_past_limit);
    assert_eq!((found[1].added, found[1].removed), (7_999, 0));
}

#[test]
fn commit_page_diff_lines__the_line_numbers_before_and_after_follow_the_hunk_headers() {
    let output = "diff --git a/a.txt b/a.txt\nindex 1..2 100644\n--- a/a.txt\n+++ b/a.txt\n\
                  @@ -10,3 +20,3 @@ fn parse\n keep\n-old\n+new\n keep\n\\ No newline at end of file\n\
                  @@ -50 +60,2 @@\n x\n+y\n";
    let found = files(&[modified("a.txt")], output, LINE_LIMIT);
    let numbers: Vec<(LineKind, Option<usize>, Option<usize>)> = found[0]
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| (line.kind, line.before, line.after))
        .collect();
    assert_eq!(
        numbers,
        [
            (LineKind::Context, Some(10), Some(20)),
            (LineKind::Removed, Some(11), None),
            (LineKind::Added, None, Some(21)),
            (LineKind::Context, Some(12), Some(22)),
            (LineKind::NoNewline, None, None),
            (LineKind::Context, Some(50), Some(60)),
            (LineKind::Added, None, Some(61)),
        ]
    );
    assert_eq!(found[0].hunks[0].header, "@@ -10,3 +20,3 @@ fn parse");
    assert_eq!(found[0].hunks[0].lines[1].text, "old");
    assert_eq!((found[0].added, found[0].removed), (2, 1));
}

#[test]
fn commit_page_diff_lines__a_binary_file_has_no_hunks_and_says_it_is_binary() {
    let output =
        "diff --git a/image.png b/image.png\nindex 1..2 100644\nBinary files a/image.png and b/image.png differ\n";
    let found = files(&[modified("image.png")], output, LINE_LIMIT);
    assert!(found[0].is_binary);
    assert_eq!(found[0].hunks, Vec::new());
    let patch = "diff --git a/image.png b/image.png\nindex 1..2 100644\nGIT binary patch\nliteral 3\nKcmZQzWB>pF\n\n\
                 literal 0\nHcmV?d00001\n\n";
    assert!(files(&[modified("image.png")], patch, LINE_LIMIT)[0].is_binary);
}

#[test]
fn commit_page_diff_lines__a_file_whose_type_changed_is_one_file_of_two_parts() {
    let listed = [FileChange {
        path: String::from("link"),
        original_path: None,
        kind: ChangeKind::TypeChanged,
    }];
    let output = "diff --git a/link b/link\ndeleted file mode 100644\nindex eb5a316..0000000\n--- a/link\n+++ /dev/null\n\
                  @@ -1 +0,0 @@\n-target\ndiff --git a/link b/link\nnew file mode 120000\nindex 0000000..1de5659\n\
                  --- /dev/null\n+++ b/link\n@@ -0,0 +1 @@\n+target\n\\ No newline at end of file\n";
    let found = files(&listed, output, LINE_LIMIT);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].hunks.len(), 2);
    assert_eq!((found[0].added, found[0].removed), (1, 1));
}

#[test]
fn commit_page_diff_lines__a_rename_and_quoted_names_are_matched_to_the_files_listed() {
    let listed = [
        FileChange {
            path: String::from("new name.txt"),
            original_path: Some(String::from("old name.txt")),
            kind: ChangeKind::Renamed,
        },
        modified("tab\tname.txt"),
    ];
    let output = "diff --git a/old name.txt b/new name.txt\nsimilarity index 100%\nrename from old name.txt\n\
                  rename to new name.txt\ndiff --git \"a/tab\\tname.txt\" \"b/tab\\tname.txt\"\nindex 1..2 100644\n\
                  --- \"a/tab\\tname.txt\"\n+++ \"b/tab\\tname.txt\"\n@@ -1 +1 @@\n-t\n+u\n";
    let found = files(&listed, output, LINE_LIMIT);
    assert_eq!(found[0].original_path.as_deref(), Some("old name.txt"));
    assert_eq!(found[0].hunks, Vec::new());
    assert_eq!(found[1].path, "tab\tname.txt");
    assert_eq!(found[1].added, 1);
}

#[test]
fn commit_page_diff_lines__an_output_that_names_other_files_is_not_shown() {
    let output = diff_of("a.txt", 1, 1);
    let unmatched = |listed: &[FileChange], output: &str| {
        let input = DiffOutput {
            files: listed,
            output: output.as_bytes(),
            limit: LINE_LIMIT,
        };
        diff_lines(&input) == PatchLines::Unmatched
    };
    assert!(unmatched(&[modified("b.txt")], &output));
    assert!(unmatched(&[modified("a.txt"), modified("b.txt")], &output));
    assert!(unmatched(&[], &output));
    assert!(unmatched(
        &[modified("a.txt")],
        &format!("warning: something\n{output}")
    ));
    assert!(unmatched(&[modified("a.txt")], &output.replace("+line 0", "?line 0")));
    assert!(!unmatched(&[], ""));
}
