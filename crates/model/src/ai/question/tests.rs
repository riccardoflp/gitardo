use crate::conflicts::{Side, SideNames, SideRole};
use crate::review::ReviewConflict;

use super::{QuestionInput, conflict_question, lines_around};

/// A merge of `feature` into `main`, the current side's commit being "Fix the parser".
fn merge_sides() -> SideNames {
    SideNames {
        current: Side {
            role: SideRole::BranchCheckedOut,
            name: Some(String::from("main")),
            subject: Some(String::from("Fix the parser")),
        },
        incoming: Side {
            role: SideRole::BranchBeingMerged,
            name: Some(String::from("feature")),
            subject: None,
        },
    }
}

/// A file of `count` lines `line N`, with a conflict whose first marker is at line `at` (from 1) and which has a
/// line on each side; the lines of the file around it are those of the file.
fn file_with_conflict(count: usize, at: usize, base: Option<Vec<String>>) -> (Vec<String>, ReviewConflict) {
    let mut lines: Vec<String> = (1..at).map(|number| format!("line {number}")).collect();
    lines.extend(
        ["<<<<<<< HEAD", "ours();", "=======", "theirs();", ">>>>>>> feature"]
            .iter()
            .map(|line| (*line).to_owned()),
    );
    let end = lines.len();
    lines.extend((end + 1..=count).map(|number| format!("line {number}")));
    let conflict = ReviewConflict {
        index: 0,
        fingerprint: String::from("f"),
        line: at,
        span: at - 1..end,
        current: vec![String::from("ours();")],
        base,
        incoming: vec![String::from("theirs();")],
        rule: None,
        stretch: None,
    };
    (lines, conflict)
}

/// The question about the conflict of a file.
fn question(lines: &[String], conflict: &ReviewConflict) -> String {
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let sides = merge_sides();
    conflict_question(&QuestionInput {
        path: "src/app.rs",
        sides: &sides,
        lines: &lines,
        conflict,
    })
}

/// Lines `line N`, each ending with a line break.
fn numbered(numbers: std::ops::Range<usize>) -> String {
    numbers.fold(String::new(), |text, number| text + &format!("line {number}\n"))
}

/// The text between a tag and its closing tag.
fn part<'a>(question: &'a str, tag: &str) -> Option<&'a str> {
    let start = question.find(&format!("<{tag}>\n"))? + tag.len() + 3;
    let end = question.get(start..)?.find(&format!("</{tag}>"))?;
    question.get(start..start + end)
}

#[test]
fn ai_conflict_question__the_40_lines_before_and_after_a_conflict_at_line_100() {
    let (lines, conflict) = file_with_conflict(300, 100, None);
    let asked = question(&lines, &conflict);
    let before: String = numbered(60..100);
    assert_eq!(part(&asked, "before"), Some(before.as_str()));
    let after: String = numbered(105..145);
    assert_eq!(part(&asked, "after"), Some(after.as_str()));
    assert_eq!(part(&asked, "current"), Some("ours();\n"));
    assert_eq!(part(&asked, "incoming"), Some("theirs();\n"));
    assert!(
        asked.starts_with("Resolve one merge conflict in the file src/app.rs.\n"),
        "{asked}"
    );
}

#[test]
fn ai_conflict_question__a_conflict_at_line_5_sends_lines_1_to_4() {
    let (lines, conflict) = file_with_conflict(20, 5, None);
    assert_eq!(
        part(&question(&lines, &conflict), "before"),
        Some("line 1\nline 2\nline 3\nline 4\n")
    );
    let strs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let sides = merge_sides();
    let around = lines_around(&QuestionInput {
        path: "src/app.rs",
        sides: &sides,
        lines: &strs,
        conflict: &conflict,
    });
    assert_eq!(around.before, ["line 1", "line 2", "line 3", "line 4"]);
}

#[test]
fn ai_conflict_question__the_sides_are_said_with_their_subjects() {
    let (lines, conflict) = file_with_conflict(20, 5, None);
    let asked = question(&lines, &conflict);
    assert!(
        asked.contains("\"Current\" is the branch checked out, in the commit \"Fix the parser\".\n"),
        "{asked}"
    );
    assert!(asked.contains("\"Incoming\" is the branch being merged.\n"), "{asked}");
}

#[test]
fn ai_conflict_question__without_a_base_there_is_no_base_part() {
    let (lines, conflict) = file_with_conflict(20, 5, None);
    assert!(!question(&lines, &conflict).contains("<base>"));
    let (lines, conflict) = file_with_conflict(20, 5, Some(vec![String::from("both();")]));
    assert_eq!(part(&question(&lines, &conflict), "base"), Some("both();\n"));
}

#[test]
fn ai_conflict_question__a_long_before_is_cut_to_its_last_12000_characters() {
    let (mut lines, conflict) = file_with_conflict(60, 41, None);
    for line in lines.iter_mut().take(40) {
        *line = "x".repeat(374);
    }
    let asked = question(&lines, &conflict);
    let before = part(&asked, "before").unwrap();
    let sent: String = (0..40).fold(String::new(), |text, _| text + &"x".repeat(374) + "\n");
    assert_eq!(sent.chars().count(), 15_000);
    let last: String = sent.chars().skip(3_000).collect();
    assert_eq!(before, format!("…\n{last}"));
}
