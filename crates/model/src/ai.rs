//! What the language model answers about a conflict: the answer read into the lines of a resolution, a reason and a
//! confidence (`ai.read-answer`), and the lines around the conflict it wrote again taken away (`ai.trim-answer`).

mod answer;
mod trim;

pub use answer::{AnswerError, Confidence, ConflictAnswer, read_answer, streamed_lines};
pub use trim::{TrimInput, trim_answer};

/// Whether a line is a conflict marker: 7 signs `<`, `=`, `>` or `|`, then the end of the line or a space.
fn is_marker(line: &str) -> bool {
    ['<', '=', '>', '|'].iter().any(|sign| {
        line.strip_prefix(&sign.to_string().repeat(7))
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}
