use super::*;
use crate::folder::TemporaryFolder;

#[test]
fn the_same_bytes_have_no_difference() {
    assert_eq!(first_difference(b"one\ntwo\n", b"one\ntwo\n"), None);
    assert_eq!(first_difference(b"", b""), None);
}

#[test]
fn the_first_differing_line_is_found_with_both_lines() {
    let line = |line: usize, expected: Option<&str>, actual: Option<&str>| LineDifference {
        line,
        expected: expected.map(String::from),
        actual: actual.map(String::from),
    };
    let cases = [
        (
            "one\ntwo\nthree\n",
            "one\ntoo\nthree\n",
            line(2, Some("two"), Some("too")),
        ),
        ("one\r\n", "one\n", line(1, Some("one\r"), Some("one"))),
        ("one\n", "one", line(2, Some(""), None)),
        ("one", "one\ntwo", line(2, None, Some("two"))),
        ("", "one", line(1, Some(""), Some("one"))),
    ];
    for (expected, actual, difference) in cases {
        let found = first_difference(expected.as_bytes(), actual.as_bytes());
        assert_eq!(found, Some(difference), "{expected:?} and {actual:?}");
    }
}

#[test]
fn a_mismatch_names_the_file_the_line_and_shows_a_carriage_return() {
    let folder = TemporaryFolder::new();
    let path = folder.path().join("case.expected");
    fs::write(&path, "first\r\nsecond\n").expect("the golden file is written");
    let error = check(&path, b"first\nsecond\n", Mode::Compare).expect_err("the line endings differ");
    let message = error.to_string();
    assert!(message.contains("case.expected"), "{message}");
    assert!(message.contains("at line 1"), "{message}");
    assert!(message.contains(r#""first\r""#), "{message}");
    assert!(message.contains(r#""first""#), "{message}");
}

#[test]
fn a_matching_file_passes_and_a_missing_one_says_how_to_record_it() {
    let folder = TemporaryFolder::new();
    let path = folder.path().join("case.expected");
    fs::write(&path, [0_u8, 1, 2, 255]).expect("the golden file is written");
    assert!(check(&path, &[0, 1, 2, 255], Mode::Compare).is_ok());
    let missing = check(&folder.path().join("missing.expected"), b"text", Mode::Compare);
    assert!(matches!(missing, Err(Error::GoldenMissing { .. })));
    assert!(
        missing
            .expect_err("the file is missing")
            .to_string()
            .contains("GITARDO_GOLDEN_RECORD=1")
    );
}

#[test]
fn recording_writes_the_actual_content_and_its_folders() {
    let folder = TemporaryFolder::new();
    let path = folder.path().join("nested").join("folder").join("case.expected");
    check(&path, b"recorded\n", Mode::Record).expect("the content is recorded");
    assert_eq!(fs::read(&path).expect("the file is there"), b"recorded\n");
    assert!(check(&path, b"recorded\n", Mode::Compare).is_ok());
    assert_golden(&path, "recorded\n");
}
