//! What is said beside each line of a file that shows its blame, from the format of `fileBlame.format`.

use std::sync::Arc;

use super::{BlameCommit, BlameLine};
use crate::graph::{DateTime, civil_date};

/// The format when the setting names none of the five values.
pub const DEFAULT_FORMAT: &str = "${date} ${author:20}";

/// What a line not committed yet says.
pub const NOT_COMMITTED: &str = "Not committed yet";

/// A value of a commit the format can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    /// `${date}`: the day, `YYYY-MM-DD` in local time.
    Date,
    /// `${ago}`: how long ago.
    Ago,
    /// `${author}`.
    Author,
    /// `${summary}`: the first line of the message.
    Summary,
    /// `${hash}`.
    Hash,
}

/// A part of the format.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    /// Text written as it is.
    Text(String),
    /// A value, and the most letters it may take.
    Value(Field, Option<usize>),
}

/// What [`format_line`] looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatInput<'a> {
    /// The format, as the setting has it.
    pub format: &'a str,
    /// The blame of each line; none for a line git said nothing of.
    pub lines: &'a [Option<BlameLine>],
    /// The time the labels are made, with the offset of local time.
    pub now: DateTime,
    /// Whether a line of the commit of the line above says nothing (`fileBlame.groupLines`).
    pub group_lines: bool,
}

/// The text beside each line: the format filled with the line's commit, each value as wide as its widest among the
/// commits of the file but the last, the spaces at the end taken off.
///
/// Feature: blame.format-line
#[must_use]
pub fn format_line(input: &FormatInput<'_>) -> Vec<String> {
    let parts = parts_of(input.format);
    let commits: Vec<&Arc<BlameCommit>> = input
        .lines
        .iter()
        .filter_map(|line| match line {
            Some(BlameLine::Committed(commit)) => Some(commit),
            _ => None,
        })
        .collect();
    let widths: Vec<usize> = parts
        .iter()
        .map(|part| {
            commits
                .iter()
                .map(|commit| value(part, commit, input.now).chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut above: Option<&Arc<BlameCommit>> = None;
    input
        .lines
        .iter()
        .map(|line| {
            let commit = match line {
                Some(BlameLine::Committed(commit)) => commit,
                Some(BlameLine::NotCommitted) => {
                    above = None;
                    return String::from(NOT_COMMITTED);
                }
                None => {
                    above = None;
                    return String::new();
                }
            };
            let is_grouped = input.group_lines && above.is_some_and(|known| known.id == commit.id);
            above = Some(commit);
            if is_grouped {
                String::new()
            } else {
                filled(&parts, &widths, commit, input.now)
            }
        })
        .collect()
}

/// The format filled with a commit.
fn filled(parts: &[Part], widths: &[usize], commit: &BlameCommit, now: DateTime) -> String {
    let mut text = String::new();
    for (index, (part, width)) in parts.iter().zip(widths).enumerate() {
        let value = value(part, commit, now);
        text.push_str(&value);
        if index + 1 < parts.len() && matches!(part, Part::Value(..)) {
            let length = value.chars().count();
            text.extend(std::iter::repeat_n(' ', width.saturating_sub(length)));
        }
    }
    text.trim_end().to_owned()
}

/// What a part says of a commit.
fn value(part: &Part, commit: &BlameCommit, now: DateTime) -> String {
    let (field, most) = match part {
        Part::Text(text) => return text.clone(),
        Part::Value(field, most) => (*field, *most),
    };
    let whole = match field {
        Field::Date => date(commit.author_time.0, now.utc_offset),
        Field::Ago => ago(now.moment.0 - commit.author_time.0),
        Field::Author => commit.author.clone(),
        Field::Summary => commit.summary.clone(),
        Field::Hash => return commit.id.as_str().chars().take(most.unwrap_or(7)).collect(),
    };
    match most {
        Some(most) if whole.chars().count() > most => {
            let mut cut: String = whole.chars().take(most - 1).collect();
            cut.push('…');
            cut
        }
        _ => whole,
    }
}

/// The day of a moment, `YYYY-MM-DD` in local time.
fn date(moment: i64, utc_offset: i64) -> String {
    let (year, month, day) = civil_date((moment + utc_offset).div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

/// How long ago, in the largest unit that has passed at least once.
#[must_use]
pub fn ago(seconds: i64) -> String {
    let units = [
        (365 * 86_400, "year"),
        (30 * 86_400, "month"),
        (86_400, "day"),
        (3_600, "hour"),
        (60, "minute"),
    ];
    units.iter().find(|(length, _)| seconds >= *length).map_or_else(
        || String::from("just now"),
        |(length, name)| {
            let count = seconds / length;
            let plural = if count == 1 { "" } else { "s" };
            format!("{count} {name}{plural} ago")
        },
    )
}

/// The parts of a format; the default's when it names none of the five values.
fn parts_of(format: &str) -> Vec<Part> {
    let parts = parse(format);
    if parts.iter().any(|part| matches!(part, Part::Value(..))) {
        parts
    } else {
        parse(DEFAULT_FORMAT)
    }
}

/// The parts of a format: each `${name}` or `${name:N}` of a known name, and the text between.
fn parse(format: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut rest = format;
    while let Some(start) = rest.find("${") {
        let (before, after) = rest.split_at(start);
        text.push_str(before);
        let Some((inside, next)) = after.get(2..).and_then(|inside| inside.split_once('}')) else {
            text.push_str(after);
            rest = "";
            break;
        };
        if let Some(value) = field_of(inside) {
            if !text.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut text)));
            }
            parts.push(value);
        } else {
            text.push_str("${");
            text.push_str(inside);
            text.push('}');
        }
        rest = next;
    }
    text.push_str(rest);
    if !text.is_empty() {
        parts.push(Part::Text(text));
    }
    parts
}

/// The value a `${…}` names, with its most letters.
fn field_of(inside: &str) -> Option<Part> {
    let (name, most) = match inside.split_once(':') {
        Some((name, most)) => (name, Some(most.trim().parse::<i64>().ok()?)),
        None => (inside, None),
    };
    let field = match name.trim() {
        "date" => Field::Date,
        "ago" => Field::Ago,
        "author" => Field::Author,
        "summary" => Field::Summary,
        "hash" => Field::Hash,
        _ => return None,
    };
    let most = most.map(|most| usize::try_from(most.max(1)).unwrap_or(1));
    Some(Part::Value(field, most))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use std::sync::Arc;

    use super::{FormatInput, ago, format_line};
    use crate::blame::{BlameCommit, BlameLine};
    use crate::graph::DateTime;
    use crate::{CommitId, Timestamp};

    const DAY: i64 = 86_400;
    /// Noon of Wednesday 11 November 2026, in UTC.
    const NOW: i64 = 1_794_398_400;

    fn commit(digit: char, author: &str, days_ago: i64, summary: &str) -> BlameLine {
        BlameLine::Committed(Arc::new(BlameCommit {
            id: CommitId::parse(&format!("abc1234{}", digit.to_string().repeat(33)))
                .unwrap_or_else(|| panic!("a hash")),
            author: author.to_owned(),
            author_time: Timestamp(NOW - days_ago * DAY),
            summary: summary.to_owned(),
            previous: None,
        }))
    }

    fn texts(format: &str, lines: &[Option<BlameLine>], group_lines: bool) -> Vec<String> {
        format_line(&FormatInput {
            format,
            lines,
            now: DateTime {
                moment: Timestamp(NOW),
                utc_offset: 3_600,
            },
            group_lines,
        })
    }

    #[test]
    fn blame_format_line__the_examples_hold_spaces_included() {
        let lines = [
            Some(commit('a', "Ann Lee", 3, "Fix the parser")),
            Some(commit('b', "Bob Stone", 60, "Add the lexer")),
        ];
        assert_eq!(
            texts("${ago} ${author:16} ${summary:40}", &lines, true),
            [
                "3 days ago   Ann Lee   Fix the parser",
                "2 months ago Bob Stone Add the lexer"
            ]
        );
        let long = [Some(commit('c', "Annabelle Fitzgerald-Smith", 3, "Fix"))];
        assert_eq!(texts("", &long, true), ["2026-11-08 Annabelle Fitzgeral…"]);
        assert_eq!(texts("${hash:4}", &long, true), ["abc1"]);
    }

    #[test]
    fn blame_format_line__an_unknown_name_is_written_as_it_is() {
        let lines = [Some(commit('a', "Ann Lee", 3, "Fix the parser"))];
        assert_eq!(texts("${nope} ${author}", &lines, true), ["${nope} Ann Lee"]);
    }

    #[test]
    fn blame_format_line__plain_text_gives_the_default() {
        let lines = [Some(commit('a', "Ann Lee", 3, "Fix the parser"))];
        assert_eq!(texts("just words", &lines, true), ["2026-11-08 Ann Lee"]);
        assert_eq!(texts("   ", &lines, true), ["2026-11-08 Ann Lee"]);
    }

    #[test]
    fn blame_format_line__three_lines_of_one_commit_grouped_say_it_once() {
        let line = commit('a', "Ann Lee", 3, "Fix the parser");
        let lines = [
            Some(line.clone()),
            Some(line.clone()),
            Some(line),
            Some(BlameLine::NotCommitted),
            None,
        ];
        assert_eq!(
            texts("${author}", &lines, true),
            ["Ann Lee", "", "", "Not committed yet", ""]
        );
        assert_eq!(texts("${author}", &lines, false)[1], "Ann Lee");
    }

    #[test]
    fn blame_format_line__how_long_ago() {
        assert_eq!(ago(59), "just now");
        assert_eq!(ago(2 * 3_600), "2 hours ago");
        assert_eq!(ago(400 * DAY), "1 year ago");
        assert_eq!(ago(DAY), "1 day ago");
        assert_eq!(ago(-5), "just now");
    }
}
