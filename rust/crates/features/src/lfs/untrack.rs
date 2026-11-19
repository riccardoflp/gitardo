//! `lfs.untrack`: takes a pattern out of the `.gitattributes` at the top, so that the files it took are stored by git
//! from their next commit on.

use model::contract::{
    AnswerKey, Answers, Choice, Context, Danger, FollowUp, Outcome, Plan, Question, QuestionKind, Refusal, Step, Text,
    Undo,
};

/// The pattern, given by the menu of a pattern or picked.
pub const PATTERN: AnswerKey = AnswerKey("pattern");
/// The file of attributes Git LFS changes: the one at the top.
const TOP: &str = ".gitattributes";
/// Why Undo has nothing to take back.
const NOT_TAKEN_BACK: &str = "Not taken back by Undo: Track a pattern… with the same pattern does.";

/// Runs `git lfs untrack <pattern>` at the top of the repository, asking which pattern of the `.gitattributes` there
/// when none is given. Refuses without Git LFS, and a pattern of another file of attributes, which is opened instead.
///
/// Feature: lfs.untrack
#[must_use]
pub fn untrack(context: &Context, answers: &Answers) -> Outcome {
    if let Some(refusal) = super::not_installed(context) {
        return Outcome::Refuse(refusal);
    }
    let patterns = &context.repository.lfs.patterns;
    let Some(pattern) = answers.text(PATTERN).or_else(|| answers.choice(PATTERN)) else {
        let choices: Vec<Choice> = patterns
            .iter()
            .filter(|pattern| pattern.file == TOP)
            .map(|pattern| Choice {
                identifier: pattern.pattern.clone(),
                label: Text::from(pattern.pattern.as_str()),
                description: None,
            })
            .collect();
        if choices.is_empty() {
            return Outcome::Refuse(Refusal::saying(
                "No pattern of the .gitattributes at the top stores files with Git LFS.",
            ));
        }
        return Outcome::Ask(Question {
            key: PATTERN,
            kind: QuestionKind::Pick { choices },
            title: Text::from("Stop storing with Git LFS"),
            detail: None,
            default: None,
        });
    };
    if let Some(other) = patterns
        .iter()
        .find(|found| found.pattern == pattern && found.file != TOP)
        .filter(|_| {
            !patterns
                .iter()
                .any(|found| found.pattern == pattern && found.file == TOP)
        })
    {
        return Outcome::Refuse(Refusal::saying(&format!(
            "{pattern} is in {}: open that file to change it.",
            other.file
        )));
    }
    Outcome::Run(Plan {
        title: Text::from(format!("Stop storing {pattern} with Git LFS")),
        steps: vec![Step::git(
            context.repository.folder.clone(),
            ["lfs", "untrack", pattern],
        )],
        danger: Danger::None,
        undo: Undo::Impossible(Text::from(NOT_TAKEN_BACK)),
        after: vec![FollowUp::Say(Text::from(format!(
            "The files {pattern} took stay as committed until they change. Commit .gitattributes."
        )))],
        on_failure: Vec::new(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests;
