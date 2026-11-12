//! The reply of the language model, read into the lines of a resolution, a reason and a confidence.

use super::is_marker;
use crate::contract::Text;

/// How sure the language model said it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// `high`.
    High,
    /// Anything but `high` or `low`.
    Medium,
    /// `low`.
    Low,
}

/// A resolution the language model proposed for a conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictAnswer {
    /// The lines that take the place of the conflict, without what ends them; none deletes them.
    pub lines: Vec<String>,
    /// Why, in its words.
    pub reason: String,
    /// How sure it is.
    pub confidence: Confidence,
}

/// Why an answer is not a resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerError(pub Text);

impl AnswerError {
    /// An error of some words.
    fn new(words: &str) -> AnswerError {
        AnswerError(Text::from(words))
    }
}

/// The text between the last opening tag of a name and the closing tag after it.
fn last_pair<'a>(reply: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = reply.rfind(&open)? + open.len();
    let inside = reply.get(start..)?;
    let end = inside.find(&close)?;
    inside.get(..end)
}

/// The lines of the text after an opening tag: from the line after the tag to the last line break.
fn lines_of(inside: &str) -> Vec<String> {
    let Some((_, after)) = inside.split_once('\n') else {
        return Vec::new();
    };
    let Some((lines, _)) = after.rsplit_once('\n') else {
        return Vec::new();
    };
    lines
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned())
        .collect()
}

/// The reply of the model read: the last `<resolution>`, `<reason>` and `<confidence>`.
///
/// Feature: ai.read-answer
///
/// # Errors
///
/// When there is no resolution, when it has conflict markers, or when no reason is given.
pub fn read_answer(reply: &str) -> Result<ConflictAnswer, AnswerError> {
    let inside = last_pair(reply, "resolution")
        .ok_or_else(|| AnswerError::new("The language model did not answer with a resolution."))?;
    let lines = lines_of(inside);
    if lines.iter().any(|line| is_marker(line)) {
        return Err(AnswerError::new(
            "The language model answered with conflict markers in its resolution.",
        ));
    }
    let reason = last_pair(reply, "reason")
        .map(|reason| reason.split_whitespace().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    if reason.is_empty() {
        return Err(AnswerError::new("The language model gave no reason."));
    }
    let confidence = match last_pair(reply, "confidence").map(|said| said.trim().to_lowercase()) {
        Some(said) if said == "high" => Confidence::High,
        Some(said) if said == "low" => Confidence::Low,
        _ => Confidence::Medium,
    };
    Ok(ConflictAnswer {
        lines,
        reason,
        confidence,
    })
}

/// The lines of the resolution written so far while the answer comes, without a closing tag cut in half.
#[must_use]
pub fn streamed_lines(partial: &str) -> Vec<String> {
    const OPEN: &str = "<resolution>";
    const CLOSE: &str = "</resolution>";
    let Some(start) = partial.rfind(OPEN) else {
        return Vec::new();
    };
    let inside = partial.get(start + OPEN.len()..).unwrap_or_default();
    let Some((_, mut written)) = inside.split_once('\n') else {
        return Vec::new();
    };
    if let Some(end) = written.find(CLOSE) {
        written = written.get(..end).unwrap_or_default();
    } else if let Some(cut) = written.rfind('<')
        && CLOSE.starts_with(written.get(cut..).unwrap_or_default())
    {
        written = written.get(..cut).unwrap_or_default();
    }
    let written = written.strip_suffix('\n').unwrap_or(written);
    if written.is_empty() {
        return Vec::new();
    }
    written
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line).to_owned())
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Confidence, read_answer, streamed_lines};

    #[test]
    fn ai_read_answer__the_resolution_the_reason_and_the_confidence() {
        let read =
            read_answer("<resolution>\nx();\n</resolution><reason>Kept  both.</reason><confidence>HIGH</confidence>");
        let read = read.unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(
            (read.lines, read.reason.as_str(), read.confidence),
            (vec![String::from("x();")], "Kept both.", Confidence::High)
        );
        let unsure =
            read_answer("<resolution>\nx();\n</resolution><reason>Guess.</reason><confidence>maybe</confidence>");
        assert_eq!(unsure.map(|read| read.confidence), Ok(Confidence::Medium));
    }

    #[test]
    fn ai_read_answer__the_last_of_two_resolutions_is_read() {
        let reply = "<resolution>\nfirst();\n</resolution> or rather <resolution>\nsecond();\n</resolution>\
                     <reason>Second thoughts.</reason>";
        assert_eq!(
            read_answer(reply).map(|read| read.lines),
            Ok(vec![String::from("second();")])
        );
    }

    #[test]
    fn ai_read_answer__an_empty_resolution_is_no_lines() {
        let read = read_answer("<resolution>\n</resolution><reason>Both sides removed it.</reason>");
        assert_eq!(read.map(|read| read.lines), Ok(Vec::new()));
        let missing = read_answer("I would keep both.");
        assert_eq!(
            missing.map_err(|error| error.0.as_str().to_owned()),
            Err(String::from("The language model did not answer with a resolution."))
        );
        let no_reason = read_answer("<resolution>\nx();\n</resolution><reason>  </reason>");
        assert_eq!(
            no_reason.map_err(|error| error.0.as_str().to_owned()),
            Err(String::from("The language model gave no reason."))
        );
    }

    #[test]
    fn ai_read_answer__a_resolution_with_markers_is_refused() {
        let read = read_answer("<resolution>\na();\n=======\nb();\n</resolution><reason>Both.</reason>");
        assert_eq!(
            read.map_err(|error| error.0.as_str().to_owned()),
            Err(String::from(
                "The language model answered with conflict markers in its resolution."
            ))
        );
    }

    #[test]
    fn ai_read_answer__a_streamed_piece_shows_its_lines_without_a_tag_cut_in_half() {
        assert_eq!(streamed_lines("<resolution>\na();\nb();\n</resol"), ["a();", "b();"]);
        assert_eq!(streamed_lines("<resolution>\na();\nb"), ["a();", "b"]);
        assert_eq!(streamed_lines("<resolu"), Vec::<String>::new());
        assert_eq!(streamed_lines("<resolution>\na();\n</resolution><reas"), ["a();"]);
    }
}
