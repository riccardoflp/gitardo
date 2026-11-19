#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::pointer;

/// The id of a content.
const OID: &str = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";

/// The three lines `git lfs` writes for a content of so many bytes.
fn written(size: u64) -> String {
    format!("version https://git-lfs.github.com/spec/v1\noid sha256:{OID}\nsize {size}\n")
}

#[test]
fn lfs_pointer__the_three_lines_of_a_file_of_2000_bytes_give_its_id_and_size() {
    let found = pointer(written(2000).as_bytes()).unwrap();
    assert_eq!(found.oid, OID);
    assert_eq!(found.size, 2000);
    assert_eq!(found.others, Vec::<(String, String)>::new());
}

#[test]
fn lfs_pointer__crlf_another_order_or_an_id_of_63_digits_is_not_a_pointer() {
    assert_eq!(pointer(written(2000).replace('\n', "\r\n").as_bytes()), None);
    let reordered = format!("version https://git-lfs.github.com/spec/v1\nsize 2000\noid sha256:{OID}\n");
    assert_eq!(pointer(reordered.as_bytes()), None);
    let short = written(2000).replace(OID, &OID[1..]);
    assert_eq!(pointer(short.as_bytes()), None);
}

#[test]
fn lfs_pointer__text_that_starts_as_a_pointer_and_goes_past_1024_bytes_is_not_one() {
    let long = format!("{}x-padding {}\n", written(2000), "a".repeat(1100));
    assert_eq!(pointer(long.as_bytes()), None);
    assert_eq!(pointer(b""), None, "an empty file is an empty file");
}

/// A diff of one file of these lines, each `<mark><text>`, `+` added, `-` removed and ` ` kept.
fn patch_of(lines: &[&str]) -> crate::commit_page::FilePatch {
    use crate::commit_page::{DiffLine, FilePatch, Hunk, LineKind};
    FilePatch {
        path: String::from("logo.psd"),
        original_path: None,
        kind: crate::state::ChangeKind::Modified,
        hunks: vec![Hunk {
            header: String::from("@@ -1,3 +1,3 @@"),
            lines: lines
                .iter()
                .map(|line| DiffLine {
                    kind: match line.chars().next() {
                        Some('+') => LineKind::Added,
                        Some('-') => LineKind::Removed,
                        _ => LineKind::Context,
                    },
                    text: line.get(1..).unwrap_or_default().to_owned(),
                    before: None,
                    after: None,
                })
                .collect(),
        }],
        added: 0,
        removed: 0,
        is_binary: false,
        is_past_limit: false,
        stored_with_lfs: None,
    }
}

#[test]
fn lfs_show_in_pages__the_diff_of_two_pointers_gives_the_sizes_of_their_contents_and_another_diff_none() {
    let other = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2394";
    let change = super::pointer_change(&patch_of(&[
        " version https://git-lfs.github.com/spec/v1",
        &format!("-oid sha256:{OID}"),
        "-size 2000",
        &format!("+oid sha256:{other}"),
        "+size 2100",
    ]));
    assert_eq!(
        change,
        Some(super::PointerChange {
            before: Some(2000),
            after: Some(2100)
        })
    );
    let added = written(12);
    let lines: Vec<String> = added.lines().map(|line| format!("+{line}")).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_eq!(
        super::pointer_change(&patch_of(&lines)),
        Some(super::PointerChange {
            before: None,
            after: Some(12)
        })
    );
    assert_eq!(super::pointer_change(&patch_of(&[" fn main() {}", "-a", "+b"])), None);
}
