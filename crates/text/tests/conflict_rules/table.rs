//! The worked examples of the card, made with real git: each row merged with git's default style and with `diff3`.

use text::conflicts::RuleCase;

use crate::check::{assert_left_as_git_wrote, assert_lines_of_the_sides, ended, rules, with_resolutions};
use crate::merges::{Case, Style, merged, merged_in_styles};

/// Lines, each ended by a line feed.
fn text(lines: &[&str]) -> String {
    ended(lines)
}

/// A case from the lines of its base and its two sides.
fn case(base: &[&str], current: &[&str], incoming: &[&str]) -> Case {
    Case {
        base: Some(text(base)),
        current: text(current),
        incoming: text(incoming),
    }
}

/// A row of the card: the three versions, and the file expected once its conflicts are resolved with the case of each,
/// or none for a conflict left.
struct Row {
    /// Why the conflict is resolved or left, as the card says.
    name: &'static str,
    /// The base and the two sides.
    case: Case,
    /// The file expected; none when it is left as git wrote it.
    expected: Option<&'static str>,
    /// The case of the conflict resolved.
    resolved_as: Option<RuleCase>,
}

/// The rows of the card that git makes as they are.
fn rows() -> Vec<Row> {
    let mut rows = resolved_rows();
    rows.extend(left_rows());
    rows
}

/// A row resolved.
fn row(name: &'static str, case: Case, expected: &'static str, resolved_as: RuleCase) -> Row {
    Row {
        name,
        case,
        expected: Some(expected),
        resolved_as: Some(resolved_as),
    }
}

/// A row left as git marked it.
fn left(name: &'static str, case: Case) -> Row {
    Row {
        name,
        case,
        expected: None,
        resolved_as: None,
    }
}

/// The rows of the card resolved.
fn resolved_rows() -> Vec<Row> {
    vec![
        row(
            "lines next to each other",
            case(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]),
            "a\nB\nC\n",
            RuleCase::NextToEachOther,
        ),
        row(
            "an addition beside a change",
            case(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "new", "c"]),
            "a\nB\nnew\nc\n",
            RuleCase::NextToEachOther,
        ),
        row(
            "a line both added among lines each changed",
            case(&["x", "y"], &["X", "new", "y"], &["x", "new", "Y"]),
            "X\nnew\nY\n",
            RuleCase::NextToEachOther,
        ),
        row(
            "one side added more of the same",
            case(&["a", "b"], &["a", "x", "y", "b"], &["a", "x", "b"]),
            "a\nx\ny\nb\n",
            RuleCase::OneSide,
        ),
        row(
            "spaces at the end on one side",
            case(
                &["one", "total = a + b", "two"],
                &["one", "total = a + b   ", "two"],
                &["one", "total = a + b + c", "two"],
            ),
            "one\ntotal = a + b + c\ntwo\n",
            RuleCase::SpacesOrLineEndings,
        ),
        row(
            "indented with a tab and with spaces",
            case(
                &["f() {", "call();", "}"],
                &["f() {", "\tcall();", "}"],
                &["f() {", "    call();", "}"],
            ),
            "f() {\n\tcall();\n}\n",
            RuleCase::SpacesOrLineEndings,
        ),
        row(
            "the same lines, other line endings",
            Case {
                base: Some(text(&["a", "b", "c"])),
                current: String::from("a\nB\r\nc\n"),
                incoming: text(&["a", "B", "c"]),
            },
            "a\nB\r\nc\n",
            RuleCase::SpacesOrLineEndings,
        ),
    ]
}

/// The rows of the card left as git marked them.
fn left_rows() -> Vec<Row> {
    let braces = ["}", "}", "}", "}", "}"];
    vec![
        left(
            "two different changes to the same line",
            case(&["a", "b", "c"], &["a", "B1", "c"], &["a", "B2", "c"]),
        ),
        left(
            "a line one side removed and the other changed",
            case(&["a", "b", "c"], &["a", "c"], &["a", "B", "c"]),
        ),
        left(
            "lines both added at the same place",
            case(&["a", "b"], &["a", "x", "b"], &["a", "y", "b"]),
        ),
        left(
            "a space that parts a word",
            case(&["x", "a=0", "y"], &["x", "a=1", "y"], &["x", "a = 1", "y"]),
        ),
        left(
            "lines that repeat",
            case(
                &braces,
                &["}", "}", "end();", "}", "}"],
                &["}", "end();", "}", "}", "}"],
            ),
        ),
        left(
            "a line that would be written twice",
            case(&["p", "q"], &["P", "log();", "q"], &["p", "Q", "log();"]),
        ),
        left(
            "line endings on one side, a real change on the other",
            Case {
                base: Some(text(&["p", "q", "r"])),
                current: String::from("p\r\nq\r\nr\r\n"),
                incoming: text(&["p", "Q", "r"]),
            },
        ),
    ]
}

/// Checks what the rules do with a file git wrote for a row.
fn check(row: &Row, style: Style, file: &str) {
    let conflicts = rules(file, &row.case);
    let cases: Vec<Option<RuleCase>> = conflicts
        .iter()
        .map(|conflict| conflict.resolution.as_ref().map(|resolution| resolution.case))
        .collect();
    assert_eq!(cases, [row.resolved_as], "{} ({style:?}):\n{file}", row.name);
    let resolved = with_resolutions(file, &conflicts);
    assert_eq!(resolved, row.expected.unwrap_or(file), "{} ({style:?})", row.name);
    assert_lines_of_the_sides(file, &conflicts);
    assert_left_as_git_wrote(file, &conflicts);
}

#[test]
fn conflicts_resolve_by_rules__each_row_of_the_card_made_with_git() {
    let rows = rows();
    let cases: Vec<Case> = rows.iter().map(|row| row.case.clone()).collect();
    let styles = [Style::Merge, Style::Diff3];
    for (style, files) in styles.iter().zip(merged_in_styles(&cases, &styles)) {
        for (row, file) in rows.iter().zip(&files) {
            check(row, *style, file);
        }
    }
    let same = "a\n<<<<<<< HEAD\nB\n=======\nB\n>>>>>>> f\nc\n";
    let both = case(&["a", "b", "c"], &["a", "B", "c"], &["a", "B", "c"]);
    let conflicts = rules(same, &both);
    assert_eq!(with_resolutions(same, &conflicts), "a\nB\nc\n");
    assert_eq!(
        conflicts[0].resolution.as_ref().map(|resolution| resolution.case),
        Some(RuleCase::SameChange)
    );
    let broken = "a\n<<<<<<< HEAD\nB\nc\n=======\nb\nC\n=======\nD\n>>>>>>> feature\n";
    let next = case(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    assert_eq!(with_resolutions(broken, &rules(broken, &next)), broken);
}

#[test]
fn conflicts_resolve_by_rules__a_line_edited_by_hand_beside_the_conflict_leaves_it() {
    let beside = case(&["a", "b", "c"], &["a", "B", "c"], &["a", "b", "C"]);
    let file = merged(std::slice::from_ref(&beside), Style::Merge).remove(0);
    let edited = file.replacen("a\n", "A\n", 1);
    assert_eq!(with_resolutions(&edited, &rules(&edited, &beside)), edited);
    let further = case(
        &["t", "u", "v", "a", "b", "c"],
        &["t", "u", "v", "a", "B", "c"],
        &["t", "u", "v", "a", "b", "C"],
    );
    let file = merged(std::slice::from_ref(&further), Style::Merge).remove(0);
    let edited = file.replacen("t\n", "T\n", 1);
    assert_eq!(
        with_resolutions(&edited, &rules(&edited, &further)),
        "T\nu\nv\na\nB\nC\n"
    );
}

#[test]
fn conflicts_resolve_by_rules__the_base_is_the_index_one_whatever_the_style_and_empty_with_none() {
    let rows = rows();
    let cases: Vec<Case> = rows.iter().map(|row| row.case.clone()).collect();
    let all = merged_in_styles(&cases, &Style::ALL);
    for (index, row) in rows.iter().enumerate() {
        let results: Vec<String> = all
            .iter()
            .map(|files| {
                let file = &files[index];
                if row.expected.is_some() {
                    with_resolutions(file, &rules(file, &row.case))
                } else {
                    String::from("left")
                }
            })
            .collect();
        assert_eq!(
            results,
            [results[0].clone(), results[0].clone(), results[0].clone()],
            "{}",
            row.name
        );
    }
    let added = Case {
        base: None,
        current: text(&["x", "y", "z"]),
        incoming: text(&["x", "z"]),
    };
    for style in Style::ALL {
        let file = merged(std::slice::from_ref(&added), style).remove(0);
        assert!(file.contains("<<<<<<<"), "{style:?}:\n{file}");
        assert_eq!(with_resolutions(&file, &rules(&file, &added)), "x\ny\nz\n", "{style:?}");
    }
}
