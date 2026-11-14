//! Rules R6.2 and R12.3: Rust files of at most 500 lines, and text in UTF-8 with LF line endings.

use super::Workspace;
use crate::files::is_rust;
use crate::problem::Problem;

/// The most lines a Rust file may have.
const MOST_LINES: usize = 500;

/// The problems of the size and the line endings of the files of the workspace.
pub fn check(workspace: &Workspace) -> Vec<Problem> {
    let mut found = Vec::new();
    for (file, text) in &workspace.files {
        let Some(text) = text else {
            continue;
        };
        if is_rust(file) {
            let lines = text.lines().count();
            if lines > MOST_LINES {
                let message = format!("The file has {lines} lines, more than {MOST_LINES}. Split it by what it does.");
                found.push(Problem::in_file("R6.2", &file.relative, message));
            }
        }
        found.extend(encoding_problem(text).map(|message| Problem::in_file("R12.3", &file.relative, message)));
    }
    found
}

/// What is wrong with the encoding or the line endings of a text, if anything. The text was read with every byte
/// that is not UTF-8 replaced by U+FFFD.
pub fn encoding_problem(text: &str) -> Option<String> {
    if text.contains('\u{FFFD}') {
        Some(String::from(
            "The file is not in UTF-8, or has a replacement character in it.",
        ))
    } else if text.contains('\r') {
        Some(String::from("The file has CR line endings; the project uses LF."))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::encoding_problem;

    #[test]
    fn wants_utf_8_and_lf() {
        assert_eq!(encoding_problem("a\nb è\n"), None);
        assert!(encoding_problem("a\r\nb\r\n").is_some());
        assert!(encoding_problem("a \u{FFFD}\n").is_some());
    }
}
