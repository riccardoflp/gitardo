#![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

use super::{LeftAlone, RuleCase, RuleConflict, RuleResolutions, RulesInput, resolve_by_rules};
use crate::conflicts::split_lines;

/// The file and the three versions git keeps, one line per item, each ended by a line feed.
struct Texts<'t> {
    file: &'t [&'t str],
    base: &'t [&'t str],
    current: &'t [&'t str],
    incoming: &'t [&'t str],
}

/// Lines joined, each ended by a line feed.
fn joined(lines: &[&str]) -> String {
    lines.iter().fold(String::new(), |mut text, line| {
        text.push_str(line);
        text.push('\n');
        text
    })
}

/// What the rules do with some texts given as bytes.
fn resolutions<'a>(file: &'a [u8], sides: [&'a [u8]; 3]) -> RuleResolutions<'a> {
    let always = || true;
    let [base, current, incoming] = sides;
    resolve_by_rules(&RulesInput {
        file,
        base,
        current,
        incoming,
        keep_going: &always,
    })
}

/// The file with the resolutions put in, and the case of each conflict resolved, or none for one left.
fn resolve(texts: &Texts<'_>) -> (String, Vec<Option<RuleCase>>) {
    let [file, base, current, incoming] = [texts.file, texts.base, texts.current, texts.incoming].map(joined);
    let found = resolutions(
        file.as_bytes(),
        [base.as_bytes(), current.as_bytes(), incoming.as_bytes()],
    );
    let RuleResolutions::LookedInto(conflicts) = found else {
        panic!("the file was left alone: {found:?}");
    };
    (with_resolutions(&file, &conflicts), cases(&conflicts))
}

/// The case of each conflict, or none for one left.
fn cases(conflicts: &[RuleConflict<'_>]) -> Vec<Option<RuleCase>> {
    conflicts
        .iter()
        .map(|conflict| conflict.resolution.as_ref().map(|resolution| resolution.case))
        .collect()
}

/// A file with the lines of each resolution put in place of those it replaces.
fn with_resolutions(file: &str, conflicts: &[RuleConflict<'_>]) -> String {
    let lines = split_lines(file);
    let mut text = String::new();
    let mut next = 0;
    for resolution in conflicts.iter().filter_map(|conflict| conflict.resolution.as_ref()) {
        for line in &lines[next..resolution.replaced.start] {
            text.push_str(line.text);
            text.push_str(line.ending);
        }
        for line in &resolution.lines {
            text.push_str(line.text);
            text.push_str(line.ending);
        }
        next = resolution.replaced.end;
    }
    for line in &lines[next..] {
        text.push_str(line.text);
        text.push_str(line.ending);
    }
    text
}

#[test]
fn conflicts_resolve_by_rules__files_too_large_binary_or_not_utf8_are_not_looked_into() {
    let marked = "<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> feature\n";
    let mut large = marked.as_bytes().to_vec();
    large.resize(2 * 1024 * 1024 + 1, b'x');
    let mut largest = marked.as_bytes().to_vec();
    largest.resize(2 * 1024 * 1024, b'x');
    let sides: [&[u8]; 3] = [b"b\n", b"B\n", b"C\n"];
    assert_eq!(
        resolutions(&large, sides),
        RuleResolutions::LeftAlone(LeftAlone::TooLarge)
    );
    assert!(matches!(resolutions(&largest, sides), RuleResolutions::LookedInto(_)));
    let with_nul = "<<<<<<< HEAD\nB\0\n=======\nC\n>>>>>>> feature\n";
    assert_eq!(
        resolutions(with_nul.as_bytes(), sides),
        RuleResolutions::LeftAlone(LeftAlone::Binary)
    );
    let latin = b"<<<<<<< HEAD\ncaf\xe9\n=======\nC\n>>>>>>> feature\n";
    assert_eq!(
        resolutions(latin, sides),
        RuleResolutions::LeftAlone(LeftAlone::NotUtf8)
    );
    assert_eq!(
        resolutions(b"a\nb\n", sides),
        RuleResolutions::LeftAlone(LeftAlone::NoMarkers)
    );
    let unpaired = "a\n<<<<<<< HEAD\nB\n=======\nC\n";
    assert_eq!(
        resolutions(unpaired.as_bytes(), sides),
        RuleResolutions::LeftAlone(LeftAlone::MarkersDoNotPair)
    );
    let many = "x\n".repeat(50_001);
    assert_eq!(
        resolutions(marked.as_bytes(), [many.as_bytes(), b"B\n", b"C\n"]),
        RuleResolutions::LeftAlone(LeftAlone::TooManyLines)
    );
}

#[test]
fn conflicts_resolve_by_rules__a_broken_conflict_between_two_is_left_and_the_others_resolved() {
    let base = ["a", "b", "c", "d", "e", "f", "g", "h", "i"];
    let current = ["a", "B", "c", "d", "E", "f", "g", "H", "i"];
    let incoming = ["a", "b", "C", "d", "E2", "f", "g", "h", "I"];
    let file = [
        "a",
        "<<<<<<< HEAD",
        "B",
        "c",
        "=======",
        "b",
        "C",
        ">>>>>>> feature",
        "d",
        "<<<<<<< HEAD",
        "E",
        "=======",
        "E2",
        "=======",
        "E3",
        ">>>>>>> feature",
        "f",
        "g",
        "<<<<<<< HEAD",
        "H",
        "i",
        "=======",
        "h",
        "I",
        ">>>>>>> feature",
    ];
    let (text, found) = resolve(&Texts {
        file: &file,
        base: &base,
        current: &current,
        incoming: &incoming,
    });
    assert_eq!(
        found,
        [Some(RuleCase::NextToEachOther), None, Some(RuleCase::NextToEachOther)]
    );
    let broken = joined(&file[9..16]);
    assert_eq!(text, format!("a\nB\nC\nd\n{broken}f\ng\nH\nI\n"));
}

#[test]
fn conflicts_resolve_by_rules__with_no_base_the_lines_both_added_are_put_together() {
    let file = ["x", "<<<<<<< HEAD", "y", "=======", ">>>>>>> feature", "z"];
    let (text, found) = resolve(&Texts {
        file: &file,
        base: &[],
        current: &["x", "y", "z"],
        incoming: &["x", "z"],
    });
    assert_eq!((text.as_str(), found), ("x\ny\nz\n", vec![Some(RuleCase::OneSide)]));
}

/// Rows of the card written by hand, with the text each resolves to and its case, or none when it is left.
const ROWS_BY_HAND: [(Texts<'static>, &str, Option<RuleCase>); 4] = [
    (
        Texts {
            file: &["a", "<<<<<<< HEAD", "B", "c", "=======", "b", "C", ">>>>>>> feature"],
            base: &["a", "b", "c"],
            current: &["a", "B", "c"],
            incoming: &["a", "b", "C"],
        },
        "a\nB\nC\n",
        Some(RuleCase::NextToEachOther),
    ),
    (
        Texts {
            file: &[
                "<<<<<<< HEAD",
                "X",
                "new",
                "y",
                "=======",
                "x",
                "new",
                "Y",
                ">>>>>>> feature",
            ],
            base: &["x", "y"],
            current: &["X", "new", "y"],
            incoming: &["x", "new", "Y"],
        },
        "X\nnew\nY\n",
        Some(RuleCase::NextToEachOther),
    ),
    (
        Texts {
            file: &["a", "x", "<<<<<<< HEAD", "y", "=======", ">>>>>>> feature", "b"],
            base: &["a", "b"],
            current: &["a", "x", "y", "b"],
            incoming: &["a", "x", "b"],
        },
        "a\nx\ny\nb\n",
        Some(RuleCase::OneSide),
    ),
    (
        Texts {
            file: &[
                "p",
                "<<<<<<< HEAD",
                "P",
                "log();",
                "q",
                "=======",
                "p",
                "Q",
                "log();",
                ">>>>>>> f",
            ],
            base: &["p", "q"],
            current: &["P", "log();", "q"],
            incoming: &["p", "Q", "log();"],
        },
        "",
        None,
    ),
];

#[test]
fn each_row_of_the_card_written_by_hand() {
    for (texts, expected, case) in ROWS_BY_HAND {
        let (text, found) = resolve(&texts);
        assert_eq!(found, [case], "{:?}", texts.file);
        if case.is_some() {
            assert_eq!(text, expected);
        } else {
            assert_eq!(text, joined(texts.file));
        }
    }
}
