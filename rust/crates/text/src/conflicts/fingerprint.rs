//! The key of a conflict that stays the same wherever it moves in the file, whatever is edited around it, and however
//! git named the sides after the markers.

use std::fmt::Write as _;

use super::sha1::sha1;

/// The key of a conflict: 16 hexadecimal digits.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fingerprint(pub String);

/// The key of a conflict, from its lines from its first marker to its last, without what ends them.
///
/// What follows `<<<<<<<`, `|||||||` or `>>>>>>>` is taken away, the lines are joined with LF and hashed with SHA-1,
/// and the key is the first 16 hexadecimal digits.
///
/// Feature: review.fingerprint
#[must_use]
pub fn fingerprint(lines: &[&str]) -> Fingerprint {
    let kept: Vec<&str> = lines
        .iter()
        .map(|line| {
            ["<<<<<<<", "|||||||", ">>>>>>>"]
                .into_iter()
                .find(|marker| line.starts_with(marker))
                .unwrap_or(line)
        })
        .collect();
    let digest = sha1(kept.join("\n").as_bytes());
    let mut key = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(key, "{byte:02x}");
    }
    Fingerprint(key)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::fingerprint;
    use crate::conflicts::{parse_markers, split_lines};

    /// The fingerprints of the conflicts of a text.
    fn fingerprints(text: &str) -> Vec<String> {
        let lines: Vec<&str> = split_lines(text).iter().map(|line| line.text).collect();
        parse_markers(text)
            .unwrap_or_default()
            .iter()
            .map(|conflict| fingerprint(lines.get(conflict.span.clone()).unwrap_or_default()).0)
            .collect()
    }

    #[test]
    fn review_fingerprint__the_names_after_the_markers_do_not_count() {
        let head = fingerprints("<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> feature\n");
        let ours = fingerprints("<<<<<<< ours\nB\n=======\nC\n>>>>>>> theirs\n");
        assert_eq!(head, ours);
        assert_eq!(head.first().map(String::len), Some(16));
    }

    #[test]
    fn review_fingerprint__a_line_added_above_does_not_change_it_and_one_changed_inside_does() {
        let conflict = "<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> f\n";
        let first = fingerprints(conflict);
        assert_eq!(fingerprints(&format!("added\n{conflict}")), first);
        assert_ne!(fingerprints(&conflict.replace('B', "b")), first);
    }

    #[test]
    fn review_fingerprint__crlf_and_lf_give_the_same() {
        let conflict = "<<<<<<< HEAD\nB\n=======\nC\n>>>>>>> f\n";
        assert_eq!(fingerprints(&conflict.replace('\n', "\r\n")), fingerprints(conflict));
    }
}
