//! The order in which `F7` goes from one changed file to the next (`diff.navigate`): by their paths, without regard to
//! case, with numbers in numeric order.

use std::cmp::Ordering;

/// Two paths compared without regard to case, a run of digits against another by its value: `b9.rs` before `b10.rs`.
#[must_use]
pub fn compare_paths(first: &str, second: &str) -> Ordering {
    let (mut first_parts, mut second_parts) = (parts(first), parts(second));
    loop {
        match (first_parts.next(), second_parts.next()) {
            (None, None) => return first.cmp(second),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(one), Some(other)) => {
                let order = compare_parts(one, other);
                if order != Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// Paths in the order of [`compare_paths`], each once.
#[must_use]
pub fn sorted_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut sorted: Vec<String> = paths.into_iter().map(str::to_owned).collect();
    sorted.sort_by(|first, second| compare_paths(first, second));
    sorted.dedup();
    sorted
}

/// A run of digits, or a run of anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part<'a> {
    /// Digits.
    Number(&'a str),
    /// Anything else.
    Text(&'a str),
}

/// Two parts compared: numbers by their value, text without regard to case, numbers before text.
fn compare_parts(first: Part<'_>, second: Part<'_>) -> Ordering {
    match (first, second) {
        (Part::Number(one), Part::Number(other)) => {
            let (one, other) = (one.trim_start_matches('0'), other.trim_start_matches('0'));
            one.len().cmp(&other.len()).then_with(|| one.cmp(other))
        }
        (Part::Number(_), Part::Text(_)) => Ordering::Less,
        (Part::Text(_), Part::Number(_)) => Ordering::Greater,
        (Part::Text(one), Part::Text(other)) => one
            .chars()
            .flat_map(char::to_lowercase)
            .cmp(other.chars().flat_map(char::to_lowercase)),
    }
}

/// The parts of a path, in order.
fn parts(path: &str) -> impl Iterator<Item = Part<'_>> {
    let mut rest = path;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let is_digit = first.is_ascii_digit();
        let end = rest
            .char_indices()
            .find(|(_, character)| character.is_ascii_digit() != is_digit)
            .map_or(rest.len(), |(index, _)| index);
        let (part, after) = rest.split_at(end);
        rest = after;
        Some(if is_digit { Part::Number(part) } else { Part::Text(part) })
    })
}
