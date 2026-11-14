//! The message of a new commit, drafted by the language model from the changes (`commit.draft-message`): what is asked
//! of it, and its answer cleaned.

use std::fmt::Write as _;

use crate::ai::AiRequest;

/// How many characters of the diff are sent.
const MOST_DIFF_CHARACTERS: usize = 20_000;

/// What the message is drafted from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftInput<'a> {
    /// The diff: of what is staged when something is, of every change otherwise.
    pub diff: &'a str,
    /// The paths of the untracked files, sent when nothing is staged; none otherwise.
    pub new_files: &'a [String],
    /// The subjects of the last commits, newest first, without the draft and the stashes.
    pub subjects: &'a [String],
}

/// What is asked of the AI provider: the task, the subjects of the recent commits, the new files, then the diff, cut to
/// its first 20,000 characters when it is longer.
///
/// Feature: commit.draft-message
#[must_use]
pub fn draft_message(input: &DraftInput<'_>) -> AiRequest {
    let mut prompt = String::from(
        "Write a Git commit message for the changes below.\n\
         First line: a summary of at most 72 characters, in the imperative mood, with no trailing period.\n\
         If the change needs explaining, add a blank line and a short body that says what changed and why.\n\
         Reply with the commit message only: no code fences, no quotes, no commentary.\n",
    );
    if !input.subjects.is_empty() {
        prompt.push_str("\nMatch the style of the recent commits in this repository:\n");
        for subject in input.subjects {
            let _ = writeln!(prompt, "- {subject}");
        }
    }
    if !input.new_files.is_empty() {
        prompt.push_str("\nNew files that are not in the diff:\n");
        for path in input.new_files {
            let _ = writeln!(prompt, "- {path}");
        }
    }
    if input.diff.chars().count() > MOST_DIFF_CHARACTERS {
        let kept: String = input.diff.chars().take(MOST_DIFF_CHARACTERS).collect();
        let _ = write!(prompt, "\nDiff (truncated):\n{kept}");
    } else {
        let _ = write!(prompt, "\nDiff:\n{}", input.diff);
    }
    AiRequest { prompt }
}

/// The answer of the model as the message box takes it: without the spaces and blank lines around it, and without a
/// code fence the model put around it anyway.
#[must_use]
pub fn clean_draft(answer: &str) -> String {
    let trimmed = answer.trim();
    let mut lines: Vec<&str> = trimmed.lines().collect();
    let is_fenced = lines.len() >= 2
        && lines.first().is_some_and(|first| first.trim_end().starts_with("```"))
        && lines.last().is_some_and(|last| last.trim() == "```");
    if is_fenced {
        let _closing = lines.pop();
        let _opening = lines.remove(0);
        return lines.join("\n").trim().to_owned();
    }
    trimmed.to_owned()
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{DraftInput, clean_draft, draft_message};

    #[test]
    fn commit_draft_message__a_long_diff_is_sent_as_its_first_20000_characters() {
        let diff = "+".repeat(50_000);
        let request = draft_message(&DraftInput {
            diff: &diff,
            new_files: &[],
            subjects: &[],
        });
        let (_, sent) = request.prompt.split_once("\nDiff (truncated):\n").unwrap();
        assert_eq!(sent, "+".repeat(20_000));
        let short = draft_message(&DraftInput {
            diff: "+a\n",
            new_files: &[],
            subjects: &[],
        });
        assert!(short.prompt.ends_with("\nDiff:\n+a\n"), "{}", short.prompt);
    }

    #[test]
    fn commit_draft_message__a_fenced_answer_is_cleaned() {
        assert_eq!(clean_draft("```text\nAdd the parser\n```"), "Add the parser");
        assert_eq!(
            clean_draft("\n\n  Add the parser\n\nIt reads JSON.\n\n"),
            "Add the parser\n\nIt reads JSON."
        );
        assert_eq!(clean_draft("```\nFix it\n```"), "Fix it");
    }

    #[test]
    fn the_subjects_and_the_new_files_are_listed() {
        let subjects = [String::from("Add the parser"), String::from("Fix the reader")];
        let new_files = [String::from("new.txt")];
        let request = draft_message(&DraftInput {
            diff: "",
            new_files: &new_files,
            subjects: &subjects,
        });
        assert!(
            request.prompt.contains(
                "\nMatch the style of the recent commits in this repository:\n- Add the parser\n- Fix the reader\n"
            ),
            "{}",
            request.prompt
        );
        assert!(
            request
                .prompt
                .contains("\nNew files that are not in the diff:\n- new.txt\n"),
            "{}",
            request.prompt
        );
    }
}
