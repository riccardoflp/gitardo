//! The lines around a conflict that a model wrote again with its answer, taken away where they are as they were sent.

use super::answer::{AnswerError, ConflictAnswer};
use super::is_marker;
use crate::contract::Text;

/// What [`trim_answer`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrimInput<'a> {
    /// The lines sent as `<before>`.
    pub before: &'a [&'a str],
    /// The lines sent as `<after>`.
    pub after: &'a [&'a str],
    /// Whether a side of the conflict had lines.
    pub has_side_lines: bool,
}

/// Whether a line anchors: it has a letter, a digit or an underscore, and is not a conflict marker.
fn is_significant(line: &str) -> bool {
    line.chars()
        .any(|character| character.is_alphanumeric() || character == '_')
        && !is_marker(line)
}

/// The significant lines of some lines, each with its place, without the spaces at their ends.
fn significant<'a>(lines: impl Iterator<Item = &'a str>) -> Vec<(usize, &'a str)> {
    lines
        .map(str::trim)
        .enumerate()
        .filter(|(_, line)| is_significant(line))
        .collect()
}

/// The answer without the lines around the conflict it wrote again.
///
/// Feature: ai.trim-answer
///
/// # Errors
///
/// When nothing is left of an answer that was trimmed while a side had lines.
pub fn trim_answer(answer: ConflictAnswer, input: &TrimInput<'_>) -> Result<ConflictAnswer, AnswerError> {
    let lines: Vec<&str> = answer.lines.iter().map(String::as_str).collect();
    let start = start_after_before(&lines, input.before);
    let mut end = end_before_after(&lines, input.after, start.unwrap_or(0));
    let after_anchors = significant(input.after.iter().copied()).len();
    if let (Some(start), None, true) = (start, end, after_anchors < 2) {
        end = Some(end_at_start_of_after(&lines, input.after, start));
    }
    let is_trimmed = start.is_some() || end.is_some_and(|end| end < lines.len());
    let kept: Vec<String> = lines
        .get(start.unwrap_or(0)..end.unwrap_or(lines.len()).max(start.unwrap_or(0)))
        .unwrap_or_default()
        .iter()
        .map(|line| (*line).to_owned())
        .collect();
    if is_trimmed && input.has_side_lines && kept.iter().all(|line| line.trim().is_empty()) {
        return Err(AnswerError(Text::from(
            "The language model wrote the lines around the conflict again, and nothing in place of the conflict.",
        )));
    }
    Ok(ConflictAnswer { lines: kept, ..answer })
}

/// Where the answer starts once the lines before the conflict it wrote again are taken away.
fn start_after_before(lines: &[&str], before: &[&str]) -> Option<usize> {
    let sent = significant(before.iter().copied());
    let [.., (last_at, second), (_, last)] = sent.as_slice() else {
        return None;
    };
    let _ = last_at;
    let answered = significant(lines.iter().copied());
    let found = answered.windows(2).position(|pair| match pair {
        [(_, first), (_, next)] => first == second && next == last,
        _ => false,
    })?;
    let up_to = answered.get(..=found + 1)?;
    if !up_to
        .iter()
        .all(|(_, line)| sent.iter().any(|(_, known)| known == line))
    {
        return None;
    }
    let mut start = answered.get(found + 1)?.0 + 1;
    let (last_sent, _) = sent.last()?;
    for following in before.iter().skip(last_sent + 1) {
        if lines.get(start).is_some_and(|line| line.trim() == following.trim()) {
            start += 1;
        } else {
            break;
        }
    }
    Some(start)
}

/// Where the answer ends once the lines after the conflict it wrote again are taken away.
fn end_before_after(lines: &[&str], after: &[&str], start: usize) -> Option<usize> {
    let sent = significant(after.iter().copied());
    let [(first_at, first), (_, second), ..] = sent.as_slice() else {
        return None;
    };
    let answered: Vec<(usize, &str)> = significant(lines.iter().copied())
        .into_iter()
        .filter(|(index, _)| *index >= start)
        .collect();
    let found = answered.windows(2).position(|pair| match pair {
        [(_, one), (_, next)] => one == first && next == second,
        _ => false,
    })?;
    let from = answered.get(found..)?;
    if !from.iter().all(|(_, line)| sent.iter().any(|(_, known)| known == line)) {
        return None;
    }
    let mut end = answered.get(found)?.0;
    for leading in after.iter().take(*first_at).rev() {
        if end > start && lines.get(end - 1).is_some_and(|line| line.trim() == leading.trim()) {
            end -= 1;
        } else {
            break;
        }
    }
    Some(end)
}

/// Where the answer ends when the lines after were too few to be found: its empty lines at the end go, then the longest
/// run of its last lines that is the start of the lines after.
fn end_at_start_of_after(lines: &[&str], after: &[&str], start: usize) -> usize {
    let mut end = lines.len();
    while end > start && lines.get(end - 1).is_some_and(|line| line.trim().is_empty()) {
        end -= 1;
    }
    let first = after
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(after.len());
    let wanted = after.get(first..).unwrap_or_default();
    let longest = (1..=wanted.len().min(end - start)).rev().find(|count| {
        let tail = lines.get(end - count..end).unwrap_or_default();
        let head = wanted.get(..*count).unwrap_or_default();
        tail.iter().zip(head).all(|(line, known)| line.trim() == known.trim())
    });
    end - longest.unwrap_or(0)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{TrimInput, trim_answer};
    use crate::ai::{Confidence, ConflictAnswer};

    fn trimmed(before: &[&str], after: &[&str], answer: &[&str]) -> Result<Vec<String>, String> {
        let answer = ConflictAnswer {
            lines: answer.iter().map(|line| (*line).to_owned()).collect(),
            reason: String::from("Kept both."),
            confidence: Confidence::High,
        };
        trim_answer(
            answer,
            &TrimInput {
                before,
                after,
                has_side_lines: true,
            },
        )
        .map(|answer| answer.lines)
        .map_err(|error| error.0.as_str().to_owned())
    }

    #[test]
    fn ai_trim_answer__the_lines_before_written_again_go() {
        let before = ["use std::io;", "", "fn main() {", "    let a = 1;", "    let b = 2;"];
        assert_eq!(
            trimmed(&before, &[], &["    let a = 1;", "    let b = 2;", "    let c = 3;"]),
            Ok(vec![String::from("    let c = 3;")])
        );
    }

    #[test]
    fn ai_trim_answer__the_lines_after_written_again_go() {
        let after = ["    print(c);", "    return;", "}"];
        assert_eq!(
            trimmed(&[], &after, &["    let c = 3;", "    print(c);", "    return;", "}"]),
            Ok(vec![String::from("    let c = 3;")])
        );
    }

    #[test]
    fn ai_trim_answer__an_answer_with_a_line_not_sent_before_them_is_left_whole() {
        let before = ["fn main() {", "    let a = 1;", "    let b = 2;"];
        let answer = ["    let z = 0;", "    let a = 1;", "    let b = 2;", "    let c = 3;"];
        assert_eq!(
            trimmed(&before, &[], &answer),
            Ok(answer.iter().map(|line| (*line).to_owned()).collect())
        );
    }

    #[test]
    fn ai_trim_answer__an_answer_that_is_only_the_lines_around_fails() {
        let before = ["fn main() {", "    let a = 1;", "    let b = 2;"];
        let after = ["    print(c);", "    return;", "}"];
        let answer = ["    let a = 1;", "    let b = 2;", "    print(c);", "    return;", "}"];
        assert_eq!(
            trimmed(&before, &after, &answer),
            Err(String::from(
                "The language model wrote the lines around the conflict again, and nothing in place of the conflict."
            ))
        );
    }

    #[test]
    fn a_brace_after_the_lines_before_goes_too_and_a_short_after_is_found_by_its_start() {
        let before = ["if ready {", "    start();", "    run();", "} else {"];
        let after = ["}", ""];
        let answer = ["    start();", "    run();", "} else {", "    wait();", "}", ""];
        assert_eq!(trimmed(&before, &after, &answer), Ok(vec![String::from("    wait();")]));
    }
}
