//! The questions of a feature, answered from the command line.

use std::collections::BTreeMap;

use engine::Asker;
use model::contract::{Answer, Plan, Question, QuestionKind};

use crate::arguments::PlanChoice;

/// Answers the questions of a feature with the `<key>=<answer>` arguments, and confirms its plan when `--yes` was
/// given. What it could not answer is kept, to be said after.
pub struct ArgumentAsker {
    answers: BTreeMap<String, String>,
    plan_choice: PlanChoice,
    /// The question no argument answered, when there was one.
    pub unanswered: Option<Question>,
}

impl ArgumentAsker {
    /// An asker with the answers of the command line.
    pub fn new(answers: BTreeMap<String, String>, plan_choice: PlanChoice) -> Self {
        ArgumentAsker {
            answers,
            plan_choice,
            unanswered: None,
        }
    }
}

impl Asker for ArgumentAsker {
    fn ask(&mut self, question: &Question) -> Option<Answer> {
        let answer = self
            .answers
            .get(question.key.0)
            .and_then(|given| answer(&question.kind, given));
        if answer.is_none() {
            self.unanswered = Some(question.clone());
        }
        answer
    }

    fn confirm(&mut self, _plan: &Plan) -> bool {
        self.plan_choice == PlanChoice::RunConfirmed
    }

    /// The text of the clipboard is the argument `clipboard=<text>`, since the command line has no window.
    fn clipboard(&mut self) -> Option<String> {
        self.answers.get("clipboard").cloned()
    }
}

/// What an argument answers to a question of this kind: a confirmation is answered only by `yes`.
fn answer(kind: &QuestionKind, given: &str) -> Option<Answer> {
    match kind {
        QuestionKind::Text { .. } | QuestionKind::Secret { .. } => Some(Answer::Text(given.to_owned())),
        QuestionKind::Pick { .. } => Some(Answer::Choice(given.to_owned())),
        QuestionKind::PickMany { .. } => Some(Answer::Choices(
            given
                .split(',')
                .filter(|part| !part.is_empty())
                .map(str::to_owned)
                .collect(),
        )),
        QuestionKind::Confirm { .. } => (given == "yes").then_some(Answer::Confirmed),
        QuestionKind::OpenFile(_) | QuestionKind::SaveFile(_) => Some(Answer::Path(std::path::PathBuf::from(given))),
    }
}

#[cfg(test)]
mod tests {
    use super::answer;
    use model::contract::{Answer, QuestionKind, Text};

    #[test]
    fn a_confirmation_is_answered_only_by_yes() {
        let confirm = QuestionKind::Confirm {
            accept: Text::from("Stage all and commit"),
        };
        assert_eq!(answer(&confirm, "yes"), Some(Answer::Confirmed));
        assert_eq!(answer(&confirm, "no"), None);
    }

    #[test]
    fn many_choices_are_separated_by_commas() {
        let many = QuestionKind::PickMany { choices: Vec::new() };
        assert_eq!(
            answer(&many, "a,b"),
            Some(Answer::Choices(vec![String::from("a"), String::from("b")]))
        );
    }
}
